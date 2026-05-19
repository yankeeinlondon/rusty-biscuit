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
| IR State | `no changes` |
| bt CLI | `—` |
| `will_use_tree_renderer` | `true` |
| `will_use_tree_renderer_with_features` | `true` |

## Current State

TextBlock is a uniformly styled block of text. Its bespoke terminal renderer wraps a plain `String` in ANSI escape sequences for bold, dim, italic, strikethrough, blink, underline, foreground color, and background color. It carries a `Layout` for margins and alignment.

Fields on the struct today:

| Field | Type | Maps to |
|-------|------|---------|
| `content` | `String` | `NodeKind::Text` value inside a `NodeKind::Paragraph` |
| `font_weight` | `FontWeight` (Normal / Bold / Dim) | `Style.emphasis.bold` / `Style.emphasis.dim` |
| `fg_color` | `Option<Color>` | `Style.color` |
| `bg_color` | `Option<Color>` | `Style.background` |
| `italic` | `bool` | `Style.emphasis.italic` |
| `strikethrough` | `bool` | `Style.emphasis.strikethrough` |
| `blink` | `bool` | `Style.emphasis.blink` |
| `underline` | `UnderliningRequest` | `Style.emphasis.underline` (mapped to `UnderlineStyle`) |
| `layout` | `Layout` | `NodeAttrs::layout()` on the projected node |

Every field has a direct, lossless mapping into the existing `renderable::style::Style` and `renderable::layout::Layout` primitives. The tree model already handles the combination (`Paragraph` + `Text` + `Style` + `Layout`) — `bt block` exercises this exact path today.

## Design Steps

### Terminal IR Implementation

- The **TextBlock** component does not currently have a IR based rendering solution
- This section will describe what is required to ensure that the **TextBlock** component:
    - has an IR implementation
    - the IR implementation drives the TerminalRenderable contract
    - the IR implementation is what is used by the bt CLI (note: TextBlock doesn't yet have a bt CLI subcommand; it will be designed below in the bt CLI section)

#### Tree Projection

TextBlock projects into the canonical render tree as:

```
Paragraph [Style, Layout]
  └─ Text { value: <content> }
```

- **Root node:** `NodeKind::Paragraph` with a single `NodeKind::Text` child.
- **Style:** A `renderable::style::Style` constructed from the component's styling fields. `fg_color` → `Style.color`, `bg_color` → `Style.background`, emphasis flags → `Style.emphasis`. Only non-default values are recorded.
- **Layout:** The component's existing `Layout` is recorded on the paragraph node via `node.attrs.set_layout(&self.layout)` when it differs from `Layout::default()`.

This projection is lossless: every styling field the bespoke renderer uses has a direct counterpart in `Style` / `Layout`, and the terminal tree renderer already lowers those into SGR sequences and margin application.

The projection mirrors what `bt block` already builds inline — a `RenderNode::paragraph(vec![RenderNode::text(...)])` with `Style` and `Layout` attached. TextBlock as a component simply encapsulates that pattern behind a typed builder API.

#### Field Mapping Detail

| TextBlock field | `Style` / `Layout` target | Notes |
|-----------------|---------------------------|-------|
| `font_weight = Bold` | `emphasis.bold = true` | |
| `font_weight = Dim` | `emphasis.dim = true` | |
| `font_weight = Normal` | (default, not set) | |
| `fg_color = Some(c)` | `color = Some(TargetValue::universal(PerMode::universal(c)))` | `Color` converts directly; `biscuit_terminal::Color` and `renderable::color::Color` are the same type |
| `bg_color = Some(c)` | `background = Some(TargetValue::universal(PerMode::universal(c)))` | Same wrapping as `fg_color` |
| `italic = true` | `emphasis.italic = true` | |
| `strikethrough = true` | `emphasis.strikethrough = true` | |
| `blink = true` | `emphasis.blink = true` | |
| `underline = Straight` | `emphasis.underline = Some(UnderlineStyle::Straight)` | All `UnderliningRequest` variants map to `UnderlineStyle` variants |
| `underline = Double` | `emphasis.underline = Some(UnderlineStyle::Double)` | |
| `underline = Curly` | `emphasis.underline = Some(UnderlineStyle::Curly)` | |
| `underline = Dotted` | `emphasis.underline = Some(UnderlineStyle::Dotted)` | |
| `underline = Dashed` | `emphasis.underline = Some(UnderlineStyle::Dashed)` | |
| `layout` | `node.attrs.set_layout(&layout)` | When non-default |

#### TreeRenderable Implementation

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

A private `build_style(&self) -> Style` helper maps the component's fields to a `Style` struct, following the same pattern used by `BlockQuote`.

#### TerminalRenderable Delegation

Once the tree projection and parity test are in place, the bespoke `render()` and `render_optimistic()` implementations are re-pointed to delegate through the tree:

```rust
fn render(&self, term: &Terminal) -> String {
    let node = self.render_tree();
    let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
    render_terminal_node(&node, &opts)
        .map(|r| r.output)
        .unwrap_or_else(|_| self.bespoke_render(term))
}
```

The old bespoke path is retained as `bespoke_render()` as a fallback during the transition period.

#### Test Variants

**Parity tests** (matching the BlockQuote parity discipline):

| # | Variant | Asserts |
|---|---------|---------|
| 1 | Plain text, no styling | Tree output equals bespoke output (both plain) |
| 2 | Bold only | Tree output contains same SGR codes as bespoke |
| 3 | Dim only | Same as above for dim SGR |
| 4 | Italic only | Same, checking `\x1b[3m` |
| 5 | Bold + italic combined | Tree output has both SGR codes |
| 6 | Foreground color (basic) | Tree output contains `\x1b[31m` etc. |
| 7 | Foreground color (RGB) | Tree output contains `\x1b[38;2;…m` |
| 8 | Background color | Tree output contains `\x1b[48;…m` |
| 9 | Underline (straight) | Tree output contains `\x1b[4m` |
| 10 | Underline (curly) | Tree output contains `\x1b[4:3m` |
| 11 | Strikethrough | Tree output contains `\x1b[9m` |
| 12 | All styles combined | Tree output has all expected SGR codes |
| 13 | Layout with left margin | Both outputs have same left padding |
| 14 | Layout with alignment center | Both outputs centered identically |
| 15 | Empty content | Both produce empty string |
| 16 | Unicode content | Content preserved in both paths |

**Tree structure tests:**

| # | Variant | Asserts |
|---|---------|---------|
| 17 | Plain text projection | Root is `Paragraph`, single child is `Text` |
| 18 | Style on node | `node.attrs.style()` returns the expected emphasis/color |
| 19 | Layout on node | `node.attrs.layout()` returns the component's layout |
| 20 | Default layout not recorded | When `Layout::default()`, no layout on node attrs |

**Cross-target tests** (once Browser and Markdown are implemented):

| # | Variant | Asserts |
|---|---------|---------|
| 21 | Markdown render | `render_markdown_node` produces plain text (no colors) |
| 22 | Browser render | `render_browser_node` produces `<p>` with inline styles |

#### Feature Requests for Tree Rendering

None. TextBlock's needs — a styled paragraph with layout — are already well-served by the existing `Paragraph` + `Text` + `Style` + `Layout` primitives. The `bt block` command exercises this exact combination today through the tree renderer. No new tree-renderer features are required.

#### Tree Renderer Fit

The existing tree renderer is a strong fit for TextBlock. The component is structurally simple (a paragraph of plain text) and its entire styling surface maps one-to-one onto `Style` and `Layout`. The terminal tree renderer already lowers these into ANSI SGR sequences and margin application — exactly what the bespoke `TextBlock::to_terminal()` does today. The only difference is that the bespoke path checks individual terminal capabilities (`term.supports_italic`, `term.underline_support`) inline, while the tree renderer handles that in its style-lowering pass. This is an architectural improvement, not a limitation.

I recommend using the tree renderer without reservation.

### Browser IR Implementation

- In this section we will provide a design specification for the **TextBlock** component's implementation of the BrowserRenderable trait.

TextBlock has no existing bespoke browser implementation — Browser is currently `❌` in the component table. The browser output will be derived entirely from the tree projection.

#### Design

The tree projection (`Paragraph` → `Text` + `Style` + `Layout`) is consumed by the existing `render_browser_node` renderer. No component-specific browser code is needed; the adapter path is:

1. `TextBlock` implements `TreeRenderable` (designed above), producing a `Paragraph` node with `Style` and `Layout`.
2. `BrowserTreeComponent<TextBlock>` wraps the component and implements `BrowserRenderable` by calling `render_tree()` then `render_browser_node()`.
3. The browser renderer lowers `Paragraph` to `<p>`, `Text` to the text value, `Style.emphasis` to semantic HTML wrappers (`<strong>`, `<em>`, `<s>`) and `<span style="…">` for underline/dim/blink, `Style.color` to `color: …`, and `Style.background` to `background-color: …`.
4. `Layout` is lowered to inline CSS margins, alignment, and `max-width`.

The existing `TextEmphasis::html_wrappers()` method already defines the semantic HTML mapping for bold, italic, strikethrough, underline, dim, and blink. The browser tree renderer applies these. No new browser-side code is needed for TextBlock specifically.

#### Browser Output Examples

| TextBlock config | Browser output |
|-----------------|----------------|
| Plain text | `<p>Hello world</p>` |
| Bold + red fg | `<p style="color: red"><strong>Hello</strong></p>` |
| Bold + italic | `<p><strong><em>Hello</em></strong></p>` |
| Blue fg, gray bg | `<p style="color: blue; background-color: gray">Hello</p>` |
| Underline (curly) | `<p><span style="text-decoration: underline; text-decoration-style: wavy">Hello</span></p>` |
| Strikethrough | `<p><s>Hello</s></p>` |
| With left margin | `<p style="margin-left: 4ch">Hello</p>` |

#### Test Variants

| # | Variant | Asserts |
|---|---------|---------|
| 1 | Plain text | HTML contains `<p>` with text, no inline styles |
| 2 | Bold | HTML contains `<strong>` |
| 3 | Italic | HTML contains `<em>` |
| 4 | Strikethrough | HTML contains `<s>` |
| 5 | Bold + italic | HTML contains `<strong><em>` nesting |
| 6 | Foreground color | HTML has `color:` in style attribute |
| 7 | Background color | HTML has `background-color:` in style attribute |
| 8 | Underline (straight) | HTML has `text-decoration: underline` |
| 9 | Underline (curly) | HTML has `text-decoration-style: wavy` |
| 10 | Dim | HTML has `opacity: 0.6` |
| 11 | Blink | HTML has `text-decoration: blink` |
| 12 | Layout margins | HTML has `margin-left` / `margin-right` |
| 13 | Layout alignment center | HTML has `text-align: center` |
| 14 | Empty content | HTML is `<p></p>` |
| 15 | Unicode content | Unicode characters preserved |

### Markdown IR Implementation

TextBlock will implement `MarkdownRenderable` with two output methods: `render_markdown()` for ergonomic Markdown and `render_markdown_plus()` for high-fidelity Markdown Plus.

#### Distinguishing Markdown from MarkdownPlus

For TextBlock, the divergence between Markdown and MarkdownPlus is straightforward:

- **Markdown** strips all color and styling information that cannot be expressed in pure Markdown syntax. Bold maps to `**text**`, italic to `_text_`, strikethrough to `~~text~~`. Colors, underline variants, blink, and dim have no Markdown equivalent and are dropped.
- **MarkdownPlus** preserves inline HTML for styling that Markdown cannot express. Colors become `<span style="color: red">text</span>`, backgrounds become `<span style="background-color: gray">text</span>`, underline variants become `<span style="text-decoration: underline">text</span>`, dim becomes `<span style="opacity: 0.6">text</span>`, and blink becomes `<span style="text-decoration: blink">text</span>`.

Both outputs are valid Markdown. When no colors, underline, dim, or blink are present, the two outputs are identical.

#### Mapping Table

| Style | Markdown | MarkdownPlus |
|-------|----------|-------------|
| Plain text | `Hello` | `Hello` |
| Bold | `**Hello**` | `**Hello**` |
| Italic | `_Hello_` | `_Hello_` |
| Bold + italic | `**_Hello_**` | `**_Hello_**` |
| Strikethrough | `~~Hello~~` | `~~Hello~~` |
| Foreground color (red) | `Hello` | `<span style="color: red">Hello</span>` |
| Background color (gray) | `Hello` | `<span style="background-color: gray">Hello</span>` |
| Underline (straight) | `Hello` | `<span style="text-decoration: underline">Hello</span>` |
| Underline (curly) | `Hello` | `<span style="text-decoration: underline; text-decoration-style: wavy">Hello</span>` |
| Dim | `Hello` | `<span style="opacity: 0.6">Hello</span>` |
| Blink | `Hello` | `<span style="text-decoration: blink">Hello</span>` |
| Bold + red fg | `**Hello**` | `<span style="color: red">**Hello****</span>` |
| Layout margins | Not represented in body | Not represented in body (could be in frontmatter `styles`) |

#### Implementation Approach

The tree projection already captures the complete style. The `render_markdown_node` renderer walks the tree and produces Markdown. For TextBlock, this means:

1. Build the tree node via `render_tree()`.
2. Call `render_markdown_node()` — the Markdown renderer already handles `Paragraph`, `Text`, and `Style`.
3. For Markdown target, the renderer drops color/background/underline/dim/blink (no Markdown equivalent) and keeps emphasis that maps to Markdown syntax.
4. For MarkdownPlus target, the renderer emits inline HTML for non-Markdown-representable styles.

Since the Markdown renderer is target-aware and already handles this distinction, TextBlock does not need custom Markdown rendering logic. The `MarkdownRenderable` impl delegates to the tree:

```rust
impl MarkdownRenderable for TextBlock {
    fn render_markdown(&self) -> String {
        let node = self.render_tree();
        let opts = MarkdownRenderOptions::default();
        render_markdown_node(&node, &opts)
            .map(|r| r.output)
            .unwrap_or_default()
    }

    fn render_markdown_plus(&self) -> String {
        let node = self.render_tree();
        let opts = MarkdownPlusRenderOptions::default();
        render_markdown_node(&node, &opts)
            .map(|r| r.output)
            .unwrap_or_default()
    }
}
```

> **Note:** The exact option types for Markdown vs MarkdownPlus rendering will depend on how the tree's Markdown renderer distinguishes the two targets. If a single `MarkdownRenderOptions` with a `plus: bool` flag is used instead, the impl adjusts accordingly.

#### Test Variants

| # | Variant | Markdown asserts | MarkdownPlus asserts |
|---|---------|-----------------|---------------------|
| 1 | Plain text | `Hello` | `Hello` |
| 2 | Bold | `**Hello**` | `**Hello**` |
| 3 | Italic | `_Hello_` | `_Hello_` |
| 4 | Strikethrough | `~~Hello~~` | `~~Hello~~` |
| 5 | Bold + italic | `**_Hello_**` | `**_Hello_**` |
| 6 | Red foreground | `Hello` (no color) | `style="color: red">Hello` |
| 7 | Gray background | `Hello` (no bg) | `style="background-color: gray">Hello` |
| 8 | Underline straight | `Hello` (no underline) | `style="text-decoration: underline">Hello` |
| 9 | Underline curly | `Hello` | `text-decoration-style: wavy` |
| 10 | Dim | `Hello` | `opacity: 0.6` |
| 11 | Blink | `Hello` | `text-decoration: blink` |
| 12 | Bold + red fg | `**Hello**` | `style="color: red">**Hello**` |
| 13 | All emphasis combined | `**_~~Hello~~_**` | inline HTML for color/underline + markdown for bold/italic/strike |
| 14 | Empty content | `""` | `""` |
| 15 | Identity when no color styles | output == MarkdownPlus output | (same) |

### `bt` CLI

- This specification will ensure that the **TextBlock** component:
    - has a 'bt' CLI subcommand for rendering this component
    - that the '--md' and '--html' CLI switches are available to render to Markdown and HTML targets respectively (the default render is always for the Terminal)
    - that the '--example' CLI switch is in place to provide a thoughtful example of how this command should be used with the CLI (see other working examples for a template)

#### Current State

| Aspect | Status |
|--------|--------|
| CLI command exists | No (`TextBlock` has no subcommand; `bt block` exists but is a generic tree-based command, not a TextBlock component command) |
| Render method used | N/A — no CLI command |
| Has `--md` / `--html` switches | No |
| Has `--example` switch | No |

#### Relationship to `bt block`

The existing `bt block` command already renders a text block through the tree with a `Style` and `Layout`, accepting `--fg`, `--bg`, `--bold`, `--italic`, `--underline`, `--strike`, `--border`, `--fill`, and `--fill-band` flags. It builds a `RenderNode::paragraph(vec![RenderNode::text(...)])` with style attached.

`bt text-block` (or simply reusing `bt block` as the CLI entry point for TextBlock) is architecturally the same operation. The **TextBlock component** and the **`bt block` command** share the same tree shape. The design choice is:

**Option A:** Add a dedicated `bt text-block` subcommand that instantiates a `TextBlock` component and renders it via the tree. This keeps the CLI command aligned with the component (matching the `bt quote` / `BlockQuote` pattern).

**Option B:** Point the existing `bt block` command at the `TextBlock` component. The `bt block` command would construct a `TextBlock`, call `render_tree()`, and render through the tree. This avoids a redundant CLI command.

**Recommendation:** Option A — add a `bt text-block` subcommand. While `bt block` and `bt text-block` produce the same tree shape, `bt block` is a lower-level "build a styled paragraph from flags" command, while `bt text-block` is the component's CLI surface. Keeping them separate allows `bt text-block` to add TextBlock-specific options (e.g., `--underline-style curly`) without crowding `bt block`. Additionally, `bt text-block` should support `--md` and `--html` targets, which `bt block` does not.

#### Specification

Add a `TextBlock(TextBlockArgs)` variant to the `Command` enum in `args.rs` and create a new `commands/text_block.rs` module.

**CLI flags:**

| Flag | Type | Description |
|------|------|-------------|
| `TEXT` | positional (required) | Text content; multiple values joined with spaces |
| `--example` / `-e` | flag | Render an example with the command used |
| `--bold` | flag | Bold text |
| `--dim` | flag | Dim text |
| `--italic` | flag | Italic text |
| `--strikethrough` / `--strike` | flag | Strikethrough text |
| `--underline` | flag | Straight underline |
| `--double-underline` | flag | Double underline |
| `--curly-underline` | flag | Curly (wavy) underline |
| `--blink` | flag | Blinking text |
| `--fg` | string | Foreground color (named or `#rrggbb`) |
| `--bg` | string | Background color (named or `#rrggbb`) |
| `--html` | flag | Render to HTML fragment instead of terminal |
| `--md` | flag | Render to portable Markdown instead of terminal |
| `--md-plus` | flag | Render to MarkdownPlus instead of terminal |
| Layout flags | `LayoutArgs` (flattened) | `--margin-left`, `--margin-right`, `--margin-top`, `--margin-bottom`, `--alignment` |

**Example:**

```
bt text-block "Release candidate passed" --fg green --bold --underline
```

Example output for `--example`:

```
Release candidate passed        (bold, green, underlined in terminal)

Command:
bt text-block "Release candidate passed" --fg green --bold --underline
```

**Implementation:**

1. Parse args into a `TextBlock` component via the builder API.
2. For terminal rendering: call `render_tree()` → `render_terminal_node()`.
3. For `--html`: call `render_tree()` → `render_browser_node()` → print HTML.
4. For `--md`: call `render_markdown()` on the component (which delegates to the tree's Markdown renderer).
5. For `--md-plus`: call `render_markdown_plus()` on the component.
6. For `--example`: use preset values (`--fg green --bold --underline`) and print the command.

The command follows the same pattern as `bt prose` for cross-target switches (`--html`, `--md`, `--md-plus` are mutually exclusive) and the same pattern as `bt block` for style flags.

## Acceptance Criteria Summary

1. `TextBlock` implements `TreeRenderable` with a `render_tree()` method that produces a `Paragraph(Text)` node with `Style` and `Layout`.
2. `TextBlock` implements `BrowserRenderable` (via `BrowserTreeComponent` or direct delegation to `render_browser_node`).
3. `TextBlock` implements `MarkdownRenderable` with `render_markdown()` and `render_markdown_plus()`.
4. The bespoke `TerminalRenderable` impl is re-pointed to delegate through the tree; the old path is retained as fallback.
5. A `bt text-block` CLI subcommand exists with `--bold`, `--dim`, `--italic`, `--strikethrough`, `--underline`, `--double-underline`, `--curly-underline`, `--blink`, `--fg`, `--bg`, `--html`, `--md`, `--md-plus`, `--example`, and layout flags.
6. Parity tests compare bespoke-vs-tree terminal output across all style combinations.
7. Cross-target tests cover Browser HTML and Markdown/MarkdownPlus output.
8. The component table in `renderable/docs/components.md` is updated to reflect the new state (Tree: ✅, Browser: ✅, Markdown: ✅, IR State: `both avail, tree renders`, bt CLI: `tree`).

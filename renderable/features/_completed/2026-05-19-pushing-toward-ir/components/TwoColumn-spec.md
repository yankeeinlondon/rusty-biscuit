# TwoColumn — IR Rendering Design Specification

| Property | Value |
|----------|-------|
| Component | `TwoColumn` |
| Location | `biscuit-terminal/lib/src/components/two_column.rs` |
| Kind | Block |
| Terminal | ✅ (tree path; bespoke retained for image-overlay fallback) |
| Browser | ✅ |
| Markdown | ✅ (portable Markdown collapses to sequential blocks; MarkdownPlus retains flex layout) |
| Tree | ✅ canonical `TreeRenderable` + compatibility `render_tree_node` hook |
| IR State | `both avail, tree renders` |
| bt CLI | `tree (Terminal default; --md, --md-plus, --html)` |
| `will_use_tree_renderer` | `true` |
| `will_use_tree_renderer_with_features` | `true` |

## Current State

TwoColumn arranges two pieces of content side by side in the terminal, with cursor-based positioning for overlay scenarios (inline images) and a vertical stacking fallback for narrow terminals. It already has a terminal compatibility tree projection (`TerminalRenderable::render_tree_node()`) that produces a `NodeKind::BlockQuote` carrying `ColumnsHints` — the flat child list is split at `left_count`, and all three tree renderers already recognize the column hints.

Important distinction: this is not yet canonical render-tree adoption. `TwoColumn` does not currently implement `renderable::tree::TreeRenderable`, so it cannot be consumed directly by `TreeComponent<T>` or `BrowserTreeComponent<T>`. The migration must factor the existing projection into a private helper and make both `TreeRenderable::render_tree()` and the legacy `TerminalRenderable::render_tree_node()` delegate to it.

Fields on the struct today:

| Field | Type | Maps to |
|-------|------|---------|
| `left` | `RenderableTerminalContent` | First `left_count` children of the projected `BlockQuote` carrier |
| `right` | `RenderableTerminalContent` | Remaining children of the projected `BlockQuote` carrier |
| `left_width` | `ColumnWidth` (Fixed/Percent) | `ColumnsHints.left_width` |
| `gap` | `u32` | `ColumnsHints.gap` |
| `layout` | `Layout` | `NodeAttrs::layout()` on the projected node |

The bespoke terminal renderer handles three rendering modes:
1. **Side-by-side text** — wraps each column's content independently, pads rows to equal height, joins with the gap.
2. **Cursor overlay** — when either column contains a `TerminalImage`, uses cursor save/restore (`\x1b7\x1b[s` / `\x1b[u\x1b8`) with terminal-appropriate cursor resets (WezTerm/Ghostty/iTerm2/Kitty get tailored behavior).
3. **Stacked fallback** — when the terminal is too narrow, renders columns vertically.

The tree projection returns `RenderNode::unsupported("two-column terminal image")` when either column contains a `TerminalImage`, because cursor-overlay image rendering has no render-tree representation. This is an accepted limitation. Terminal rendering must fall back to the bespoke path for that case; Browser and Markdown targets should surface the existing unsupported-node behavior according to strictness rather than trying to emulate terminal cursor overlays.

## Design Steps

### Terminal IR Implementation

- The **TwoColumn** component does not currently have an IR-based rendering solution
- This section will describe what is required to ensure that the **TwoColumn** component:
    - has an IR implementation
    - the IR implementation drives the TerminalRenderable contract
    - the IR implementation is what is used by the bt CLI (note: TwoColumn already has `bt columns` as a CLI subcommand; it will be updated below in the bt CLI section)

#### Tree Projection (Partially Exists)

The projection logic already exists in `TerminalRenderable::render_tree_node()` in `two_column.rs`. It must be factored into a private helper, for example `fn render_columns_tree(&self) -> RenderNode`, so the canonical and compatibility hooks cannot drift. It produces:

```
BlockQuote [ColumnsHints, Layout]
  ├─ <left column block nodes>    (first `left_count` children)
  └─ <right column block nodes>   (remaining children)
```

The `ColumnsHints` carry:
- `gap`: character gap between columns
- `left_width`: `ColumnWidthKind::Fixed(n)` or `ColumnWidthKind::Percent(f)`
- `left_count`: index where left children end and right children begin
- `stack_below`: whether to stack vertically below a width threshold

The terminal tree renderer already recognizes `ColumnsHints` on a `BlockQuote` and renders side-by-side columns natively.

The projection already handles:
- Wrapping inline-only content in `Paragraph` nodes (via `project_column()`)
- Terminal image columns returning `RenderNode::unsupported`
- Layout propagation when non-default

#### What Remains for Terminal IR

The projection exists and the terminal tree renderer handles it. The remaining work is:

1. **Implement canonical `TreeRenderable`.** Factor the projection helper and make `TreeRenderable::render_tree()` return the same node as `TerminalRenderable::render_tree_node()`.
2. **Flip `render()` to delegate through the tree.** The bespoke `render()` method becomes the fallback; the primary path calls the projection helper → `render_terminal_node()`.
3. **Expand parity tests** comparing bespoke-vs-tree output across the key variants, including direct bespoke output versus tree output rather than only tree semantic assertions.
4. **Accept the terminal-image limitation.** When either column contains a `TerminalImage`, the projection returns `RenderNode::unsupported`. The `render()` method must fall back to the bespoke path in this case.

#### TerminalRenderable Delegation

```rust
fn render(&self, term: &Terminal) -> String {
    let node = self.render_tree();
    if !matches!(node.kind, NodeKind::Unsupported { .. }) {
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        if let Ok(rendered) = render_terminal_node(&node, &opts) {
            return rendered.output;
        }
    }
    self.bespoke_render(term)
}
```

The old bespoke path is retained as `bespoke_render()` for:
- Fallback when the tree path fails
- The terminal-image overlay path (which has no tree representation)

#### Test Variants

**Parity tests** (matching the BlockQuote parity discipline):

| # | Variant | Asserts |
|---|---------|---------|
| 1 | Plain text, 50/50 split | Tree output matches bespoke (both side-by-side) |
| 2 | Custom ratio (70/30) | Tree output has correct column widths matching bespoke |
| 3 | Fixed left width | Tree output matches bespoke with fixed-width left column |
| 4 | Multi-line left, single-line right | Rows padded to equal height in both paths |
| 5 | Multi-line both columns | Row alignment matches between paths |
| 6 | Custom gap | Gap width matches in both outputs |
| 7 | Stacked (narrow terminal) | Both paths produce vertical stack when width ≤ gap |
| 8 | With left margin | Layout margins applied identically in both paths |
| 9 | With right margin | Same as above |
| 10 | With alignment center | Block alignment applied identically |
| 11 | Prose content in columns | Styled text content preserved in both paths |
| 12 | Component content in columns | Component rendering matches between paths. Note: nested non-`Prose` block components currently flatten to ANSI-stripped paragraph text in the projected tree (text survives, structural kind does not). See `lessons-learned.md` → "Nested non-`Prose` block components flatten to text" for the accepted limitation and Stage 3 plan. |
| 13 | TerminalImage in left column | Bespoke path used (tree returns unsupported) |
| 14 | TerminalImage in right column | Bespoke path used |
| 15 | Empty left content | Both handle gracefully |
| 16 | Empty right content | Both handle gracefully |
| 17 | Unicode content | Content preserved in both paths |

For variants 1-12 and 15-17, assertions must compare the stripped bespoke terminal output against the stripped tree-rendered terminal output on content and placement invariants. Exact byte equality is acceptable only where ANSI, wrapping, and trailing-space differences are intentionally stable. Variants 13-14 must assert that the public `render()` path uses the bespoke image overlay path and does not print the unsupported placeholder.

**Tree structure tests:**

| # | Variant | Asserts |
|---|---------|---------|
| 18 | Plain text projection | Root is `BlockQuote` with `ColumnsHints` |
| 19 | Column split | `left_count` matches expected split point |
| 20 | Gap in hints | `ColumnsHints.gap` equals component's gap |
| 21 | Left width kind | `ColumnsHints.left_width` matches component setting |
| 22 | Layout on node | `node.attrs.layout()` returns the component's layout |
| 23 | Default layout not recorded | When `Layout::default()`, no layout on node attrs |
| 24 | Terminal image returns unsupported | `render_tree_node()` returns `Some(RenderNode::unsupported(...))` |
| 25 | Canonical trait parity | `TreeRenderable::render_tree()` and `TerminalRenderable::render_tree_node()` produce the same serialized node |
| 26 | Columns hints validation | `ColumnsHints` round-trips through `NodeAttrs` and validation reports no structural errors when placed on the `BlockQuote` carrier |

#### Feature Requests for Tree Rendering

TwoColumn does not need a dedicated `NodeKind`; `ColumnsHints` on a block carrier are sufficient. It does, however, need two render-tree renderer enhancements so Browser and MarkdownPlus do not silently lose column sizing information.

##### RT-TWOCOLUMN-001: Browser CSS lowering for `ColumnsHints`

**APPROVED**

this feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: the browser renderer already recognizes `ColumnsHints`, but it currently emits only CSS-ready classes. That preserves the existence of two columns but loses the component's public width and gap contract. Because `ColumnsHints` are already target-agnostic tree data, browser lowering belongs in the render-tree renderer rather than in a bespoke `TwoColumn` browser implementation.

Required behavior:

- In `renderable/src/tree/render/browser.rs`, enhance the existing `render_columns` path for `ColumnsHints`.
- Keep the existing outer `<div class="columns">` and two `<div class="column">` children.
- Add inline CSS to the outer container for `display: flex`, `gap: {gap}ch`, and any layout CSS already derived from `NodeAttrs::layout()`. Do not overwrite layout declarations when combining style strings.
- Lower `ColumnWidthKind::Fixed(n)` on the left column to `flex: 0 0 {n}ch; max-width: {n}ch`.
- Lower `ColumnWidthKind::Percent(p)` on the left column to `flex: 0 0 {p * 100}%`, clamping `p` to `0.0..=1.0` before formatting.
- Give the right column `flex: 1 1 0` so it consumes the remaining width.
- Preserve the existing plain class hooks for external CSS.
- HTML escaping remains handled by the existing child renderers.
- Tests must cover default columns, fixed left width, percent left width, custom gap, layout plus column CSS on the same container, empty left/right columns, and normal `BlockQuote` rendering unchanged when no `ColumnsHints` are present.

##### RT-TWOCOLUMN-002: MarkdownPlus HTML lowering for `ColumnsHints`

**APPROVED**

this feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: portable Markdown has no side-by-side layout, so the existing sequential fallback is correct for `MarkdownDialect::Markdown`. MarkdownPlus is explicitly the richer dialect for inline/block HTML, and it should preserve the component's two-column layout from the same canonical tree projection instead of requiring a bespoke MarkdownPlus renderer.

Required behavior:

- In `renderable/src/tree/render/markdown.rs`, keep `MarkdownDialect::Markdown` behavior unchanged: render left blocks, a blank line when both sides are non-empty, then right blocks.
- For `MarkdownDialect::MarkdownPlus`, render a block HTML flex container equivalent to the browser shape: an outer `<div class="columns" style="display:flex;gap:{gap}ch">` with two `<div class="column">` children.
- Lower `ColumnWidthKind::Fixed(n)` and `ColumnWidthKind::Percent(p)` on the left column using the same CSS semantics as the browser renderer.
- Do not apply `Layout` in either Markdown dialect; this matches the documented Markdown layout contract in `layout-and-style.md`.
- Render child content through the Markdown renderer for the active dialect before embedding it in the HTML container. Escape text through the existing child rendering paths; do not concatenate raw `Text` node values directly into HTML.
- If a child render would produce Markdown block syntax that is unsafe inside a raw HTML block for the target Markdown parser, prefer rendering those children through the browser fragment renderer for MarkdownPlus or document the accepted parser constraint in tests. The implementation must not produce malformed HTML.
- Tests must cover portable Markdown unchanged, MarkdownPlus default columns, fixed width, percent width, custom gap, empty columns, emphasis/prose content, nested block content, and unsupported image columns under Warn and Strict.

#### Tree Renderer Fit

The existing tree renderer is a strong fit for TwoColumn. The component's projection is already implemented as a terminal compatibility hook, and the terminal tree renderer already processes `ColumnsHints` on `BlockQuote` nodes to produce side-by-side output. The mapping is direct: the component's `left_width` → `ColumnsHints.left_width`, `gap` → `ColumnsHints.gap`, `layout` → `NodeAttrs.layout()`. The terminal tree renderer handles column width resolution, row padding, gap insertion, and vertical stacking — all the behaviors the non-image bespoke renderer implements.

The one exception is cursor-overlay rendering for inline images, which the tree cannot represent. This is handled by having `render_tree_node()` return an `Unsupported` node when images are detected, and the `render()` method falls back to the bespoke path. This is the correct architectural choice: inline image rendering is inherently terminal-specific and should remain bespoke.

I recommend using the tree renderer for all non-image TwoColumn rendering once the canonical `TreeRenderable` impl and parity gate are in place.

### Browser IR Implementation

- In this section we will provide a design specification for the **TwoColumn** component's implementation of the BrowserRenderable trait.

TwoColumn has no existing bespoke browser implementation — Browser is currently `❌` in the component table. However, the browser tree renderer already handles `ColumnsHints` on `BlockQuote` nodes. It currently produces a `<div class="columns">` container holding two `<div class="column">` children and needs the approved RT-TWOCOLUMN-001 enhancement to lower width and gap hints to CSS.

#### Design

The browser output is derived entirely from the existing tree projection. No component-specific browser code is needed; the adapter path is:

1. `TwoColumn` implements `TreeRenderable::render_tree()` by delegating to the factored projection helper, producing a `BlockQuote` carrier node with `ColumnsHints` and `Layout`.
2. `BrowserTreeComponent<TwoColumn>` wraps the component and implements `BrowserRenderable` by calling `render_tree()` then `render_browser_node()`.
3. The browser renderer detects `ColumnsHints` on the `BlockQuote`, splits children at `left_count`, and wraps each group in a `<div class="column">` inside a `<div class="columns">`.
4. `Layout` is lowered to inline CSS margins, alignment, and `max-width` on the container `<div>`.

The `left_width` from `ColumnsHints` can be expressed as CSS on the left column `<div>`:
- `ColumnWidthKind::Fixed(n)` → `style="flex: 0 0 {n}ch; max-width: {n}ch"`
- `ColumnWidthKind::Percent(p)` → `style="flex: 0 0 {p*100}%"`

The `gap` from `ColumnsHints` maps to `column-gap` on the container: `style="column-gap: {gap}ch"`.

These CSS enhancements are the approved RT-TWOCOLUMN-001 render-tree work and must be applied by the existing `render_columns` method in the browser renderer, not by any component-specific code.

#### Browser Output Examples

| TwoColumn config | Browser output |
|-----------------|----------------|
| Plain text | `<div class="columns"><div class="column"><p>Left</p></div><div class="column"><p>Right</p></div></div>` |
| 70/30 split | Left `<div class="column">` gets `style="flex: 0 0 70%"` |
| Fixed left width 30 | Left `<div class="column">` gets `style="flex: 0 0 30ch"` |
| Gap 6 | Container gets `style="column-gap: 6ch"` |
| With left margin | Container gets `style="margin-left: 2ch"` |

#### Test Variants

| # | Variant | Asserts |
|---|---------|---------|
| 1 | Plain text, 50/50 | HTML contains `<div class="columns">` with two `<div class="column">` children |
| 2 | Custom ratio (70/30) | Left column has `flex: 0 0 70%` style |
| 3 | Fixed left width | Left column has `flex: 0 0 30ch` style |
| 4 | Custom gap | Container has `column-gap: 6ch` |
| 5 | With margin | Container has `margin-left` / `margin-right` |
| 6 | Prose content | Columns contain rendered HTML paragraphs |
| 7 | Empty left content | Left `<div class="column">` is present but empty |
| 8 | Terminal image in column | Tree returns unsupported; browser emits the standard unsupported fallback/diagnostic according to strictness |
| 9 | Layout plus gap | Container has both layout CSS and column gap CSS without one overwriting the other |

### Markdown IR Implementation

TwoColumn will implement `MarkdownRenderable` with two output methods: `render_markdown()` for ergonomic Markdown and `render_markdown_plus()` for high-fidelity Markdown Plus.

#### Distinguishing Markdown from MarkdownPlus

For TwoColumn, the divergence between Markdown and MarkdownPlus is significant because Markdown has no native side-by-side layout:

- **Markdown** collapses the two-column layout into sequential blocks. The left column's content is emitted first, then a blank line, then the right column's content. This is the same behavior as the existing `render_columns` method in the Markdown renderer — left blocks, blank line, right blocks. Column widths, gap, and side-by-side arrangement are lost.
- **MarkdownPlus** preserves the two-column layout using a CSS flex container expressed as block HTML. This preserves the side-by-side arrangement and can encode column widths and gap.

Both outputs are valid Markdown. When the content is simple text that would stack naturally (e.g., a title and a description), Markdown remains sequential while MarkdownPlus preserves the two-column HTML shape. They should not be expected to be byte-identical once MarkdownPlus column lowering is implemented.

#### Mapping Table

| TwoColumn config | Markdown | MarkdownPlus |
|-----------------|----------|-------------|
| Plain text 50/50 | `Left\n\nRight` | `<div class="columns" style="display:flex;gap:3ch"><div style="flex:1"><p>Left</p></div><div style="flex:1"><p>Right</p></div></div>` |
| 70/30 split | `Left\n\nRight` | Left `<div>` gets `style="flex:0 0 70%"` |
| Fixed left 30 | `Left\n\nRight` | Left `<div>` gets `style="flex:0 0 30ch"` |
| Gap 6 | `Left\n\nRight` | Container gets `style="gap:6ch"` |
| With margins | Not represented in body | Not represented in body |
| Prose content | Styled text via markdown syntax | Inline HTML for non-Markdown-representable styles |

#### Implementation Approach

The existing Markdown tree renderer already handles `ColumnsHints` on `BlockQuote` by emitting left blocks, a blank line, then right blocks. For the Markdown target, this behavior is correct and needs no change.

For the MarkdownPlus target, the renderer should produce block HTML when it encounters `ColumnsHints`. The approach:

1. Build the tree node via `TreeRenderable::render_tree()`.
2. For Markdown target: call `render_markdown_node()` with `MarkdownDialect::Markdown` — the existing `render_columns` method produces sequential blocks, which is the best ergonomic representation.
3. For MarkdownPlus target: call `render_markdown_node()` with `MarkdownDialect::MarkdownPlus`. The Markdown renderer's `render_columns` method must detect the `MarkdownPlus` dialect and emit block HTML (`<div class="columns" style="display:flex;...">`) instead of sequential blocks.

This means the `render_columns` method in the Markdown renderer needs a dialect-aware branch:

```rust
fn render_columns(&mut self, children: &[RenderNode], hints: &ColumnsHints) -> Result<String, RenderError> {
    match self.opts.dialect {
        MarkdownDialect::Markdown => { /* existing: sequential blocks */ }
        MarkdownDialect::MarkdownPlus => { /* block HTML flex container */ }
    }
}
```

Since `TwoColumn` delegates entirely to the tree, the `MarkdownRenderable` impl is:

```rust
impl MarkdownRenderable for TwoColumn {
    fn render_markdown(&self) -> String {
        let node = self.render_tree();
        let opts = MarkdownRenderOptions { dialect: MarkdownDialect::Markdown, ..Default::default() };
        render_markdown_node(&node, &opts)
            .map(|r| r.output)
            .unwrap_or_default()
    }

    fn render_markdown_plus(&self) -> String {
        let node = self.render_tree();
        let opts = MarkdownRenderOptions { dialect: MarkdownDialect::MarkdownPlus, ..Default::default() };
        render_markdown_node(&node, &opts)
            .map(|r| r.output)
            .unwrap_or_default()
    }
}
```

#### Test Variants

| # | Variant | Markdown asserts | MarkdownPlus asserts |
|---|---------|-----------------|---------------------|
| 1 | Plain text 50/50 | Left text, blank line, right text | `<div class="columns"` with flex CSS |
| 2 | Custom ratio (70/30) | Width info lost (sequential) | Left div has `flex: 0 0 70%` |
| 3 | Fixed left width | Width info lost | Left div has `flex: 0 0 30ch` |
| 4 | Custom gap | Gap lost | Container has `gap: 6ch` or `column-gap: 6ch` |
| 5 | Prose content | Bold/italic via `**`/`_` syntax | Same or inline HTML for complex styles |
| 6 | Empty left content | Only right text | Empty left div present |
| 7 | Empty right content | Only left text | Empty right div present |
| 8 | Simple text divergence | sequential Markdown output | MarkdownPlus keeps HTML columns; do not assert byte identity |
| 9 | Terminal image column | Standard unsupported behavior under Warn/Strict | Standard unsupported behavior under Warn/Strict |

### `bt` CLI

- This specification will ensure that the **TwoColumn** component:
    - has a 'bt' CLI subcommand for rendering this component
    - that the '--md' and '--html' CLI switches are available to render to Markdown and HTML targets respectively (the default render is always for the Terminal)
    - that the '--example' CLI switch is in place to provide a thoughtful example of how this command should be used with the CLI (see other working examples for a template)

#### Current State

| Aspect | Status |
|--------|--------|
| CLI command exists | Yes — `bt columns` is implemented in `cli/src/commands/columns.rs` |
| Render method used | Bespoke — calls `columns.render(&term)` directly |
| Has `--md` / `--html` switches | No |
| Has `--example` switch | Yes — `--example` / `-e` flag exists |

The existing `bt columns` command:
- Takes `LEFT` and `RIGHT` positional args (optional when `--example`)
- Supports `--gap`, `--left` (width spec), and layout flags (`--margin-left`, `--margin-right`, `--alignment`)
- Renders via `TwoColumn::render(&term)` (bespoke path)
- Has `--example` with preset values (`--gap 6 --left 18`)

#### Specification

**Add `--html`, `--md`, and `--md-plus` switches** to `ColumnsArgs` and route rendering through the tree:

**New CLI flags:**

| Flag | Type | Description |
|------|------|-------------|
| `--html` | flag | Render to HTML fragment instead of terminal |
| `--md` | flag | Render to portable Markdown instead of terminal |
| `--md-plus` | flag | Render to MarkdownPlus instead of terminal |

These three flags are mutually exclusive. When none is specified, the default is terminal rendering.

**Updated `run()` logic:**

1. Parse args into a `TwoColumn` component (same as today).
2. For terminal rendering: call `columns.render(&term)`; after the component is flipped, that public method owns the tree-first path and the bespoke fallback.
3. For `--html`: call `TreeRenderable::render_tree()` → `render_browser_node()` → print HTML.
4. For `--md`: call `render_markdown()` on the component (which delegates to the tree's Markdown renderer).
5. For `--md-plus`: call `render_markdown_plus()` on the component.
6. For `--example`: use existing preset values and print the command.

**Example output for `--example`:**

The existing `COLUMNS_EXAMPLE_CMD` already demonstrates the terminal usage. For the `--example` switch, the command continues to render a terminal example by default. If `--html` / `--md` / `--md-plus` is combined with `--example`, the example renders in that target format and the printed command includes the target switch.

**Implementation detail:** The terminal CLI call site should remain simple:

```rust
let output = columns.render(&term);
```

After the component is flipped, the CLI should continue calling the public terminal render method for the default target. The tree-first/fallback logic belongs inside `TwoColumn::render()`. Keep the old implementation in a private `bespoke_render(&self, term: &Terminal) -> String` / `bespoke_render_optimistic(&self, width: Option<u32>) -> String` helper before changing `render()` so the fallback cannot recurse.

## Acceptance Criteria Summary

1. `TwoColumn` implements canonical `TreeRenderable`; the existing `TerminalRenderable::render_tree_node()` remains only as a compatibility delegate to the same private projection helper.
2. `TwoColumn` implements `BrowserRenderable` (via `BrowserTreeComponent` or direct delegation to `render_browser_node`), producing `<div class="columns">` flex containers.
3. `TwoColumn` implements `MarkdownRenderable` with `render_markdown()` (sequential blocks) and `render_markdown_plus()` (block HTML flex container).
4. The bespoke `TerminalRenderable` impl is re-pointed to delegate through the tree; the old path is retained as fallback for terminal-image overlay scenarios.
5. The `bt columns` CLI subcommand is updated with `--html`, `--md`, and `--md-plus` target switches.
6. Parity tests compare bespoke-vs-tree terminal output across all key variants.
7. Cross-target tests cover Browser HTML and Markdown/MarkdownPlus output.
8. The component table in `renderable/docs/components.md` is updated to reflect the new state (Browser: ✅, Markdown: ✅, IR State: `both avail, tree renders`, bt CLI: `tree`).
9. Approved render-tree work RT-TWOCOLUMN-001 and RT-TWOCOLUMN-002 is completed before the component is implemented against the new behavior.

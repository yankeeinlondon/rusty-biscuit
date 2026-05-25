# BlockQuote IR Design Specification

`will_use_tree_renderer: true`
`will_use_tree_renderer_with_feature: false`

## Current Status

| Property | Value |
|----------|-------|
| Terminal | ✅ |
| Browser | ✅ |
| Markdown | ✅ |
| Tree | ✅ |
| IR State | both avail, tree renders |
| bt CLI | tree |

BlockQuote already implements `TreeRenderable` (projects to `NodeKind::BlockQuote`), and the three tree renderers (Terminal, Browser, Markdown) all handle the `BlockQuote` variant. The bespoke `TerminalRenderable` impl is still the default render path. The `bt quote` CLI command calls the bespoke `render()` path.

The existing component also has a compatibility API that is not fully expressible through the current render tree: `BlockQuote::with_border()` accepts an arbitrary terminal prefix string. The tree path can represent the semantic block quote and its typed left border style, but it cannot preserve an arbitrary prefix such as `"> "` or `"!! "` without a component-specific extension. That behavior must remain on the bespoke compatibility path unless the render-tree model grows a general-purpose mechanism later.

## Design Steps

### Terminal IR Implementation

**Status: done.** The IR State is "both avail, old renders" — both the bespoke and tree rendering paths exist. The remaining work is to **flip** the default to tree rendering, not to build the tree projection.

The flip involves re-pointing `BlockQuote`'s `TerminalRenderable::render()` and `render_optimistic()` through `TreeComponent<BlockQuote>` (or by delegating to `render_tree()` then `render_terminal_node`) for the style-compatible default path, while retaining the old bespoke code path so the parity gate continues to validate fidelity.

A parity test already exists (`render_tree_component_parity.rs`) that renders BlockQuote both ways and compares semantic equivalence. The flip should be gated on that test passing.

No tree-renderer feature requests are needed for the default terminal path — the existing `render_styled` path in the terminal renderer already handles `BlockQuote` with a declared `Style` (border, fill, color), and `render_tree()` already seeds that `Style` onto the projected node.

#### Terminal compatibility fallback

`BlockQuote::with_border()` must continue to work. Because `Style::Border` is typed appearance and does not preserve arbitrary component prefix strings, `TerminalRenderable` should route through the legacy `render_content()` path when `self.border != BlockQuote::default().border`. This keeps the public compatibility API intact while still letting the normal `BlockQuote` path migrate to the tree renderer.

Add a regression test that builds a quote with `with_border("> ")` (or another non-default prefix), renders it through `render_optimistic(Some(width))`, and asserts the custom prefix is preserved. This test should intentionally document that custom prefixes are a compatibility fallback, not a render-tree feature.

### Browser IR Implementation

- in this section we will provide a design specification for the **BlockQuote** component's implementation of the `BrowserRenderable` trait

The browser tree renderer already handles `NodeKind::BlockQuote` — it emits a `<blockquote>` element containing the rendered children (see `renderable/src/tree/render/browser.rs:206`). The `BrowserTreeComponent<T: TreeRenderable>` adapter already bridges any `TreeRenderable` into `BrowserRenderable` (see `biscuit-terminal/lib/src/render_tree/browser_adapter.rs`).

Therefore, BlockQuote's `BrowserRenderable` impl is a thin delegation:

```rust
impl BrowserRenderable for BlockQuote {
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        BrowserTreeComponent::new(self.clone()).render_html_fragment()
    }

    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
        BrowserTreeComponent::new(self.clone()).render_html_page(page)
    }

    fn as_any(&self) -> &dyn Any { self }
}
```

The `render_html_fragment` output will be `<blockquote><p>Quoted text</p></blockquote>` for a simple quote, and `<blockquote><p>Quoted text</p><p>— Attribution</p></blockquote>` for a quote with attribution. Rendering a `RenderNode::root(vec![quote.render_tree()])` directly through `render_browser_node` will instead wrap the quote in the root renderer's `<div>`. Use `BrowserTreeComponent` for the component-level `BrowserRenderable` impl when a fragment is expected.

The `Style` currently on the node (left border color, text color, background) is not yet consumed by the browser renderer — `Style` browser lowering is designed but not wired (per `layout-and-style.md` §6, "Browser Style lowering is unbuilt"). The `<blockquote>` will render with browser-default styling until browser `Style` lowering is implemented. This is acceptable as a first pass; the browser path will gain visual fidelity when the browser `Style` lowering is wired (a separate cross-component task).

#### Key test variants for Browser

1. Simple text quote → contains `<blockquote>` and text content
2. Quote with attribution → two `<p>` children, second contains `— Author`
3. Quote from Prose content → text extracted and Prose styling flattened inside `<blockquote>`
4. `render_html_page(None)` → full `<html>` page with `<body>` containing the fragment
5. Empty quote → `<blockquote>` with empty or minimal `<p>`
6. `render_html_page(Some(PageOptions))` → page options applied
7. Styled quote → contains the quote text and `<blockquote>` even though style is not yet lowered to CSS

### Markdown IR Implementation

- in this section we will provide a design specification for the **BlockQuote** component's implementation of the `MarkdownRenderable` trait

The markdown tree renderer already handles `NodeKind::BlockQuote` — it renders each line of the inner content prefixed with `> ` (see `renderable/src/tree/render/markdown.rs:196`). BlockQuote's `render_tree()` already projects the content and attribution into the canonical tree structure.

Therefore, BlockQuote's `MarkdownRenderable` impl delegates through the tree:

```rust
impl MarkdownRenderable for BlockQuote {
    fn render_markdown(&self) -> String {
        let node = self.render_tree();
        render_markdown_node(&node, &MarkdownRenderOptions::default())
            .map(|r| r.output)
            .unwrap_or_default()
    }

    fn render_markdown_plus(&self) -> String {
        self.render_markdown()
    }
}
```

#### Markdown vs MarkdownPlus divergence for BlockQuote

There is **no divergence** between Markdown and MarkdownPlus for BlockQuote because:

- BlockQuote's content is plain text (or Prose text with ANSI stripped in the tree projection).
- The `Style` on the node (border color, text color, background) represents visual appearance that has no Markdown representation. In both Markdown and MarkdownPlus, the block quote renders as `> text` lines — colors are purely terminal/browser concerns.
- Attribution is rendered as `— Author` text in a separate paragraph, which is valid Markdown.
- No inline HTML is needed for either output.

The only scenario where divergence could occur is if BlockQuote's `Style` included foreground/background colors and MarkdownPlus chose to represent them with inline HTML `<span style="color:...">`. However, the current `render_tree()` projection strips Prose styling into plain text, so this situation does not arise. If future work preserves inline styling in the tree projection, the `render_markdown_plus()` implementation should be revisited.

#### Test strategy for Markdown

1. Simple text quote → `> Quoted text`
2. Quote with attribution → `> Quoted text\n> \n> — Author`
3. Quote from Prose content → `> bold content` (styling stripped, text preserved)
4. Multiline content → each line prefixed with `> `
5. Empty quote → empty string or minimal output
6. Long text → no additional Markdown wrapping is applied by the tree renderer; the line is emitted as `> {text}` unless the source already contains line breaks
7. Verify `render_markdown()` and `render_markdown_plus()` produce identical output
8. Styled quote → Markdown output remains unchanged because `Style` is intentionally ignored by Markdown rendering
9. Custom `with_border()` quote → Markdown output uses canonical `>` block quote syntax, not the custom terminal prefix

### `bt` CLI

- this specification will ensure that the **BlockQuote** component:
    - has a 'bt' CLI subcommand for rendering this component
    - that the '--md' and '--html' CLI switches are available to render to Markdown and HTML targets respectively (the default render is always for the Terminal)
    - that the '--example' CLI switch is in place to provide a thoughtful example of how this command should be used with the CLI (see other working examples for a template)

#### Current State

| Property | Status |
|----------|--------|
| CLI command exists | Yes — `bt quote` |
| Render method | Tree renderer (`render_terminal_node`) |
| Has `--md` switch | Yes |
| Has `--html` switch | Yes |
| Has `--example` switch | Yes |
| Uses tree renderer | Yes |

#### Specification Design

1. **Switch to tree rendering for terminal output.** Replace the normal bespoke `quote.render(&term)` call with the tree path: project via `render_tree()`, wrap in `RenderNode::root`, and render through `render_terminal_node`. This matches the pattern established by `bt block` and `bt progress`. If a future CLI option exposes a custom border prefix, route that non-default case through the compatibility fallback described above.

2. **Add `--md` flag.** When set, render through `render_markdown_node` and print the Markdown output to STDOUT.

3. **Add `--html` flag.** When set, render through `BrowserTreeComponent` and print the HTML fragment to STDOUT. Avoid wrapping the component in `RenderNode::root` for this path unless the desired output is a `<div>` wrapper around the `<blockquote>`.

4. **Retain `--example` flag.** The example should demonstrate all three targets:
   - Terminal: `bt quote --attribution "Engineering Notes" "<b>Clarity</b> is kind when the work gets complex."`
   - Add `--md` example variant in the help text
   - Add `--html` example variant in the help text

5. **Update `QUOTE_EXAMPLE_CMD`** to reflect the tree-rendered command.

#### CLI flag conflicts

`--md` and `--html` are mutually exclusive with each other (a single output target per invocation). Use clap's `conflicts_with` attribute:

```rust
#[arg(long, conflicts_with = "html")]
pub md: bool,

#[arg(long, conflicts_with = "md")]
pub html: bool,
```

#### Implementation sketch

```rust
impl Run for QuoteArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        // ... build quote and apply layout ...

        if self.md {
            let root = RenderNode::root(vec![quote.render_tree()]);
            let rendered = render_markdown_node(&root, &MarkdownRenderOptions::default())
                .map_err(|e| color_eyre::eyre::eyre!("markdown render failed: {e}"))?;
            println!("{}", rendered.output);
        } else if self.html {
            let html = BrowserTreeComponent::new(quote).render_html_fragment().render();
            println!("{html}");
        } else {
            let term = detect_terminal_honoring_force_color();
            let root = RenderNode::root(vec![quote.render_tree()]);
            let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
            let rendered = render_terminal_node(&root, &opts)
                .map_err(|e| color_eyre::eyre::eyre!("render failed: {e}"))?;
            println!("{}", rendered.output);
        }

        if self.example {
            print_example_command(QUOTE_EXAMPLE_CMD);
        }
        Ok(())
    }
}
```

## Acceptance Criteria for Implementation

- [x] `BlockQuote` implements `TerminalRenderable` via the tree renderer (flip from bespoke to tree)
- [x] `BlockQuote::with_border()` keeps its existing custom-prefix behavior through a documented bespoke compatibility fallback
- [x] `BlockQuote` implements `BrowserRenderable` (delegating through `BrowserTreeComponent`)
- [x] `BlockQuote` implements `MarkdownRenderable` (delegating through `render_markdown_node`)
- [x] `bt quote` renders via the tree renderer for terminal output
- [x] `bt quote --md "text"` outputs Markdown-formatted block quote
- [x] `bt quote --html "text"` outputs HTML-formatted block quote
- [x] `bt quote --example` shows a representative example with the command that produced it
- [x] `--md` and `--html` are mutually exclusive
- [x] Parity test (`render_tree_component_parity.rs`) continues to pass after the flip
- [x] Custom-border regression test proves the compatibility fallback preserves `with_border()`
- [x] New unit tests for `BrowserRenderable` and `MarkdownRenderable` impls
- [x] CLI integration tests for `--md` and `--html` flags
- [x] Components table updated: BlockQuote Browser ❌→✅, Markdown ❌→✅, IR State → `both avail, tree renders`, bt CLI → `tree`

## Render-tree Feature Requests

### Request RT-BQ-001: Add arbitrary BlockQuote terminal prefix support to the render tree

**DENIED**

This feature will not be added to the render-tree tree implementation. You should try to still use the render-tree where practical and work around the complexity but if the complexity is too great then you have permission to create a bespoke IR implementation for this component.

Why: `BlockQuote::with_border()` is a terminal-specific compatibility API that accepts arbitrary prefix strings. The render tree should keep `NodeKind::BlockQuote` semantic and target-agnostic, while visual border behavior belongs in typed `Style::Border`. Adding a custom prefix field would encode a single component's terminal presentation quirk into the canonical tree model. The practical implementation path is to use the tree renderer for the normal styled block quote and retain a bespoke compatibility fallback for non-default custom prefixes.

### Request RT-BQ-002: Require Browser `Style` lowering before BlockQuote can implement `BrowserRenderable`

**DENIED**

This feature will not be added to the render-tree tree implementation. You should try to still use the render-tree where practical and work around the complexity but if the complexity is too great then you have permission to create a bespoke IR implementation for this component.

Why: browser `Style` lowering is an acknowledged cross-component gap in `layout-and-style.md`, but it is not required for semantic BlockQuote browser output. The browser renderer already emits correct `<blockquote>` structure and text. BlockQuote should adopt the tree-backed browser path now, with visual parity improving later when browser `Style` lowering lands for all styled components.

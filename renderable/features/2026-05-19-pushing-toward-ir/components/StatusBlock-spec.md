# StatusBlock IR Design Specification

`will_use_tree_renderer: true`
`will_use_tree_renderer_with_feature: true`

## Current Status

| Property | Value |
|----------|-------|
| Terminal | ✅ |
| Browser | ❌ |
| Markdown | ❌ |
| Tree | ❌ |
| IR State | no changes |
| bt CLI | — |

StatusBlock is a composite component that wraps three sub-renderables: a `Status` header line (severity icon + text), a `BlockQuote` body (severity-colored left border), and an optional `Prose` hint. All three are rendered via bespoke terminal-only paths. StatusBlock has no `TreeRenderable` implementation, no browser or markdown rendering, and no `bt` CLI subcommand.

## Design Steps

### Terminal IR Implementation

- The **StatusBlock** component does not currently have a IR based rendering solution
- This section will describe what is required to ensure that the **StatusBlock** component:
    - has an IR implementation
    - the IR implementation drives the TerminalRenderable contract
    - the IR implementation is what is used by the bt CLI (note if **StatusBlock** doesn't yet have bt CLI subcommand then it will be designed below in the bt CLI section)

#### Tree projection strategy

StatusBlock has no direct `NodeKind` variant (the 25-variant vocabulary does not include a "StatusBlock" or "Callout" kind). The projection composes existing node kinds to represent the three visual sections:

```
Root [Layout: margins, word-wrap]
  ├── Paragraph [Style: severity color]     // header
  │   └── Text "✗ Shell expansion failed"
  ├── BlockQuote [Style: left border severity color]  // body
  │   └── Paragraph
  │       └── Text "Missing closing brace"
  └── Paragraph                              // hint
      └── Text "Check the template syntax and retry."
```

Each section is only present when the corresponding field is non-empty. A body-only StatusBlock produces just the `BlockQuote`; a header-only block produces just a `Paragraph`.

**Status icon resolution.** The `Status` component's icon rendering is terminal-specific (Nerd Font detection, theme selection, color-depth adaptation). At tree-projection time the icon is pre-resolved against a default `Terminal::new_optimistic(80)` and embedded as plain text — the same lossy-but-pragmatic approach `BlockQuote` uses when extracting text from `Prose`. The severity color rides on `Style.color` for the header paragraph and `Style.border.color` for the body block quote, so the terminal tree renderer lowers them through `render_styled`.

**Body text extraction.** The body is a `Vec<Prose>`. Each `Prose` is rendered optimistically and ANSI-stripped (via `strip_escape_codes`), then joined with `\n\n` separators into a single text block inside the `BlockQuote`. This matches the bespoke renderer's visual behavior (stacked paragraphs separated by blank lines inside a continuous border) while accepting the same styling loss as `BlockQuote::plain_text()`.

**Layout and Style mapping.**

| StatusBlock field | Tree target |
|---|---|
| `layout.margin` | Root node `NodeAttrs::set_layout` |
| `layout.word_wrap` | Root node `NodeAttrs::set_layout` (carried through to children) |
| `severity` → default color | Header `Style.color`, body `BlockQuote` `Style.border.color` |
| `border_color` override | Same two Style slots, overriding severity default |
| `border` glyph | Not carried on the tree — the terminal tree renderer's `BlockQuote` lowering draws its own `│ ` border from `Style.border`. The bespoke `border` field (`┃ ` default) is a StatusBlock-specific concern that diverges from the tree renderer's border. This is an accepted visual divergence (see parity test design). |

#### Implementation sketch

```rust
impl TreeRenderable for StatusBlock {
    fn render_tree(&self) -> RenderNode {
        use renderable::style::{Border, BorderSides, PerMode, Style};
        use renderable::tree::RenderNode;

        let severity_color = self.resolved_border_color();
        let mut children = Vec::new();

        if let Some(ref header_text) = self.header {
            let status = Status::from_prose(header_text).state(self.severity.clone());
            let term = Terminal::new_optimistic(80);
            let rendered_header = strip_escape_codes(&status.render(&term));
            let mut header_node = RenderNode::paragraph(vec![RenderNode::text(rendered_header)]);
            header_node.attrs.set_style(&Style {
                color: Some(TargetValue::universal(PerMode::universal(severity_color.clone()))),
                ..Style::default()
            });
            children.push(header_node);
        }

        if !self.body.is_empty() {
            let body_text: String = self.body.iter()
                .map(|p| strip_escape_codes(&p.render_optimistic(None)))
                .collect::<Vec<_>>()
                .join("\n\n");
            let mut bq_node = RenderNode::block_quote(vec![
                RenderNode::paragraph(vec![RenderNode::text(body_text)])
            ]);
            bq_node.attrs.set_style(&Style {
                border: Some(Border {
                    color: Some(TargetValue::universal(PerMode::universal(severity_color))),
                    sides: BorderSides::left_only(),
                    ..Border::default()
                }),
                ..Style::default()
            });
            children.push(bq_node);
        }

        if let Some(ref hint_text) = self.hint {
            children.push(RenderNode::paragraph(vec![RenderNode::text(hint_text.clone())]));
        }

        let mut root = RenderNode::root(children);
        if self.layout != StatusBlock::new(StatusState::NotStarted).layout {
            root.attrs.set_layout(&self.layout);
        }
        root
    }
}
```

#### Parity test design

A new parity test file (`biscuit-terminal/lib/tests/status_block_parity.rs`) follows the pattern established by `render_tree_component_parity.rs`:

**Semantic token parity** (bespoke vs. `TreeComponent<StatusBlock>`):
1. Header-only: status icon text + header words survive both paths
2. Body-only: body text survives, bordered on both paths
3. Body with Prose content: words survive (styling loss accepted)
4. Header + body: both sections' words survive
5. Header + body + hint: all three sections' words survive
6. Multiple body Prose items: all items' words survive
7. Custom border color: colored output on both paths (SGR present)
8. Margins applied: layout content appears indented on both paths

**Accepted divergences** (same discipline as BlockQuote parity):
- **Border glyph**: Bespoke uses `┃ ` (StatusBlock default); tree renderer uses `│ ` (BlockQuote native). Both are valid left-border glyphs; the visual difference is accepted.
- **Prose styling is flattened** in the tree projection (same as BlockQuote).
- **Status icon rendering**: The bespoke path resolves the icon against the actual `Terminal` passed to `render()`; the tree path resolves it at projection time against `Terminal::new_optimistic(80)`. In a no-color terminal the icon character matches; with Nerd Fonts the tree path may use the fallback icon since the optimistic terminal does not detect Nerd Fonts.

#### Feature Requests for Tree Rendering

No feature requests are needed. The existing 25-variant `NodeKind` vocabulary adequately represents a StatusBlock as a composition of `Root` + `Paragraph` + `BlockQuote` + `Paragraph`. The severity state is encoded at projection time through `Style` (color, border) and pre-resolved icon text. This is the same pragmatic approach `BlockQuote` uses for text extraction and works well for a composite component.

A future `NodeKind::Callout { severity, children }` variant would allow each renderer to apply target-native treatment (e.g., browser CSS classes, markdown emoji prefixes) rather than pre-rendering terminal-biased icon text. This is a nice-to-have optimization, not a blocker.

#### Recommendation

The existing tree renderer is a good fit for StatusBlock. The composition of `Root`, `Paragraph`, and `BlockQuote` nodes captures the structural intent, `Style` carries the severity-driven coloring, and `Layout` handles the margins. The only accepted loss is Prose styling in the body (already an established precedent) and the border glyph difference (`┃` vs `│`). I recommend using the tree renderer.

### Browser IR Implementation

- in this section we will provide a design specification for the **StatusBlock** component's implementation of the `BrowserRenderable` trait

StatusBlock has no existing browser rendering implementation. The tree projection designed above produces a `Root` node containing `Paragraph` and `BlockQuote` children. The browser tree renderer already handles all three node kinds (`Root`, `Paragraph`, `BlockQuote`), so the `BrowserRenderable` impl is a thin delegation through `BrowserTreeComponent`:

```rust
impl BrowserRenderable for StatusBlock {
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        BrowserTreeComponent::new(self.clone()).render_html_fragment()
    }

    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
        BrowserTreeComponent::new(self.clone()).render_html_page(page)
    }

    fn as_any(&self) -> &dyn Any { self }
}
```

The rendered output for a full StatusBlock (Error severity, all parts present):

```html
<p>✗ Shell expansion failed</p>
<blockquote><p>Missing closing brace</p></blockquote>
<p>Check the template syntax and retry.</p>
```

The severity color encoded on `Style` is not yet consumed by the browser renderer (browser `Style` lowering is designed but not wired — per `layout-and-style.md` §6). The browser output will use default styling until that cross-component work lands.

#### Key test variants for Browser

1. Body-only StatusBlock → `<blockquote>` present, no `<p>` before or after
2. Header + body → `<p>` with icon/header text, then `<blockquote>`
3. Header + body + hint → three elements: `<p>`, `<blockquote>`, `<p>`
4. Header-only (empty body, no hint) → single `<p>` with icon text
5. `render_html_page(None)` → full `<html>` page wrapping the fragment
6. All severity states (Error, Warning, Info, Success, Active) → text content reflects correct icon
7. Custom border color → `Style` carried on the tree (visual enforcement deferred to browser `Style` lowering)

### Markdown IR Implementation

- in this section we will provide a design specification for the **StatusBlock** component's implementation of the `MarkdownRenderable` trait

The markdown tree renderer already handles `Root`, `Paragraph`, and `BlockQuote`. StatusBlock's `MarkdownRenderable` impl delegates through the tree:

```rust
impl MarkdownRenderable for StatusBlock {
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

#### Markdown vs MarkdownPlus divergence for StatusBlock

There is **no divergence** between Markdown and MarkdownPlus for StatusBlock because:

- The tree projection carries no inline color styling that would require `<span style="color:...">` in MarkdownPlus. The severity colors are on `Style.color` and `Style.border`, which the Markdown renderer ignores entirely (per `layout-and-style.md` §4).
- The status icon is embedded as plain text (e.g., `✗`, `⚠`, `ℹ`) — valid in both Markdown and MarkdownPlus without inline HTML.
- The body is projected as a `BlockQuote` containing plain text — standard Markdown.
- The hint is projected as a `Paragraph` of plain text — standard Markdown.
- No inline HTML is needed for either output target.

If future work preserves Prose styling in the tree projection (e.g., `<red>error</red>` → `<span style="color:red">error</span>` in MarkdownPlus), the `render_markdown_plus()` implementation should be revisited. For now, the Prose styling is flattened at projection time, matching the BlockQuote precedent.

#### Example Markdown output

For `StatusBlock::new(Error).header("<b>Shell expansion failed</b>").body("Missing closing brace").hint("Check syntax")`:

```markdown
✗ Shell expansion failed

> Missing closing brace

Check syntax
```

#### Test strategy for Markdown

1. Body-only → single block quote: `> Body text`
2. Header + body → paragraph with icon text, blank line, block quote
3. Header + body + hint → three sections separated by blank lines
4. All severity states → correct fallback icon character present in output
5. Multiple body Prose items → all items' text inside the block quote
6. Empty body (header + hint only) → two paragraphs, no block quote
7. Verify `render_markdown()` and `render_markdown_plus()` produce identical output
8. Custom border color → no effect on markdown output (color is not rendered)

### `bt` CLI

- this specification will ensure that the **StatusBlock** component:
    - has a 'bt' CLI subcommand for rendering this component
    - that the '--md' and '--html' CLI switches are available to render to Markdown and HTML targets respectively (the default render is always for the Terminal)
    - that the '--example' CLI switch is in place to provide a thoughtful example of how this command should be used with the CLI (see other working examples for a template)

#### Current State

| Property | Status |
|----------|--------|
| CLI command exists | No |
| Render method | N/A |
| Has `--md` switch | No |
| Has `--html` switch | No |
| Has `--example` switch | No |
| Uses tree renderer | No |

StatusBlock has no `bt` CLI subcommand. The component is currently only used programmatically via its `TerminalRenderable` implementation.

#### Specification Design

1. **Add `bt status-block` subcommand.** Register in `args.rs` as a new variant in the `Command` enum, following the established display-order convention. Create a new command module at `cli/src/commands/status_block.rs`.

2. **Render via tree for all targets.** The command projects StatusBlock to a `RenderNode` via `render_tree()`, wraps it in `RenderNode::root`, and dispatches to the appropriate renderer:
   - Default (terminal): `render_terminal_node`
   - `--md`: `render_markdown_node`
   - `--html`: `render_browser_node`

3. **Add `--md` and `--html` flags.** Mutually exclusive, matching the BlockQuote spec pattern.

4. **Add `--severity` flag.** Required (unless `--example`), accepts `error`, `warning`, `info`, `success`, `active`, `not-started`, `tool-use`, `subagent`. Maps to `StatusState`.

5. **Add `--header` flag.** Optional prose-formatted header text.

6. **Add `--hint` flag.** Optional hint text.

7. **Add `--border-color` flag.** Optional named color or `#rrggbb` override for the severity-derived border color.

8. **Positional body text.** Multiple values joined with spaces, treated as the body content.

9. **Add `--example` flag.** Shows a representative example with all parts populated.

#### CLI argument sketch

```rust
#[derive(ClapArgs, Debug, Clone)]
pub struct StatusBlockArgs {
    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(long, value_enum, required_unless_present = "example")]
    pub severity: Option<StatusSeverityArg>,

    #[arg(long)]
    pub header: Option<String>,

    #[arg(long)]
    pub hint: Option<String>,

    #[arg(long = "border-color")]
    pub border_color: Option<String>,

    #[arg(value_name = "BODY", required_unless_present = "example")]
    pub body: Vec<String>,

    #[arg(long, conflicts_with = "html")]
    pub md: bool,

    #[arg(long, conflicts_with = "md")]
    pub html: bool,

    #[command(flatten)]
    pub layout: LayoutArgs,
}
```

#### Example constants

```rust
const STATUS_BLOCK_EXAMPLE_CMD: &str =
    r#"bt status-block --severity error --header "<b>Shell Expansion Failed</b>" --hint "Check the template syntax and retry." "Missing closing brace in `${...}` directive.""#;
```

#### Implementation sketch

```rust
impl Run for StatusBlockArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let severity = self.severity
            .or_else(|| self.example.then_some(StatusSeverityArg::Error))
            .ok_or_else(|| color_eyre::eyre::eyre!("--severity is required"))?;

        let state = severity.to_status_state();
        let body_text = if self.example {
            "Missing closing brace in `${...}` directive.".to_string()
        } else {
            self.body.join(" ")
        };

        let mut block = StatusBlock::new(state);

        let header = self.header
            .or_else(|| self.example.then(|| "<b>Shell Expansion Failed</b>".to_string()));
        if let Some(h) = header {
            block = block.header(h);
        }

        if !body_text.is_empty() {
            block = block.body(body_text);
        }

        let hint = self.hint
            .or_else(|| self.example.then(|| "Check the template syntax and retry.".to_string()));
        if let Some(h) = hint {
            block = block.hint(h);
        }

        if let Some(color) = &self.border_color {
            block = block.border_color(parse_color(color)?);
        }

        // Apply layout
        if let Some(left) = self.layout.margin_left {
            block = block.left_margin(TargetValue::universal(Length::ch(left)));
        }
        if let Some(right) = self.layout.margin_right {
            block = block.right_margin(TargetValue::universal(Length::ch(right)));
        }

        let node = block.render_tree();
        let root = RenderNode::root(vec![node]);

        if self.md {
            let rendered = render_markdown_node(&root, &MarkdownRenderOptions::default())
                .map_err(|e| color_eyre::eyre::eyre!("markdown render failed: {e}"))?;
            println!("{}", rendered.output);
        } else if self.html {
            let rendered = render_browser_node(&root, &BrowserRenderOptions::default())
                .map_err(|e| color_eyre::eyre::eyre!("browser render failed: {e}"))?;
            println!("{}", rendered.output.render());
        } else {
            let term = detect_terminal_honoring_force_color();
            let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
            let rendered = render_terminal_node(&root, &opts)
                .map_err(|e| color_eyre::eyre::eyre!("render failed: {e}"))?;
            println!("{}", rendered.output);
        }

        if self.example {
            print_example_command(STATUS_BLOCK_EXAMPLE_CMD);
        }
        Ok(())
    }
}
```

## Acceptance Criteria for Implementation

- [ ] `StatusBlock` implements `TreeRenderable` (projects to `Root` + `Paragraph` + `BlockQuote` composition)
- [ ] `StatusBlock` implements `TerminalRenderable` via the tree renderer (`TreeComponent<StatusBlock>`)
- [ ] `StatusBlock` implements `BrowserRenderable` (delegating through `BrowserTreeComponent`)
- [ ] `StatusBlock` implements `MarkdownRenderable` (delegating through `render_markdown_node`)
- [ ] `bt status-block` command registered with `--severity`, `--header`, `--body`, `--hint`, `--border-color` flags
- [ ] `bt status-block --md` outputs Markdown-formatted output
- [ ] `bt status-block --html` outputs HTML-formatted output
- [ ] `bt status-block --example` shows a representative example with all parts and the command that produced it
- [ ] `--md` and `--html` are mutually exclusive
- [ ] Parity test (`status_block_parity.rs`) validates semantic content survives both bespoke and tree paths
- [ ] Unit tests for `BrowserRenderable` and `MarkdownRenderable` impls
- [ ] Unit tests for `TreeRenderable` projection (node structure, Style, Layout)
- [ ] CLI integration tests for `--md`, `--html`, `--example`, and all severity states
- [ ] Components table updated: StatusBlock Tree ❌→✅, Browser ❌→✅, Markdown ❌→✅, IR State → `both avail, old renders`, bt CLI → `tree`

# StatusBlock IR Design Specification

`will_use_tree_renderer: true`
`will_use_tree_renderer_with_feature: false`

## Current Status

| Property | Value |
|----------|-------|
| Terminal | yes |
| Browser | yes |
| Markdown | yes |
| Tree | yes |
| IR State | tree default + bespoke compatibility fallback |
| bt CLI | tree |

StatusBlock is a composite component that wraps three sub-renderables: a
`Status` header line, a `BlockQuote` body, and an optional `Prose` hint. It is
currently terminal-only. The existing implementation preserves a few terminal
compatibility details that should be treated carefully during tree adoption:

- `StatusBlock::new()` defaults to a right margin of `5` cells and
  `WordWrap::WrapProse(Some(8), None)`.
- The body border defaults to `"┃ "`, which maps cleanly to a thick left
  `Style::Border`.
- `StatusBlock::border(String)` accepts an arbitrary terminal prefix. That is
  not target-agnostic and must not be promoted into the canonical render tree.
- Header and hint strings are prose-formatted today; the first tree projection
  may flatten that styling to text, matching the existing `BlockQuote`
  component migration precedent.

Success means StatusBlock has one canonical `TreeRenderable` projection used by
the new Markdown and Browser render paths, and by the terminal render path when
the component is using target-agnostic settings. The arbitrary custom border
prefix remains a compatibility-only escape hatch.

## Terminal IR Implementation

### Tree projection strategy

StatusBlock does not need a dedicated `NodeKind`. The existing node vocabulary
can represent it as a composition of a root container, an optional header
paragraph, an optional block quote body, and an optional hint paragraph:

```text
Root [Layout: component layout, classes: status-block status-block--error]
  ├── Paragraph [Style: severity color, classes: status-block__header]
  │   └── Text "⤫ Shell expansion failed"
  ├── BlockQuote [Style: thick left border in severity color, classes: status-block__body]
  │   └── Paragraph
  │       └── Text "Missing closing brace"
  └── Paragraph [classes: status-block__hint]
      └── Text "Check the template syntax and retry."
```

Only present sections should be emitted. A body-only block emits a root with one
`BlockQuote`; a header-only block emits a root with one `Paragraph`.

The root node must always carry the component layout directly in `NodeAttrs`
because `StatusBlock::new()` has non-empty default layout. Do not rely only on
`TreeRenderable::tree_layout()`: the current `TreeComponent` and
`BrowserTreeComponent` adapters render the node returned by `render_tree()` and
do not add the optional hook afterward.

### Status and severity mapping

The status icon should be resolved to the stable Unicode fallback form at tree
projection time, not to Nerd Font-specific glyphs. Use an explicit no-color,
non-Nerd terminal or an equivalent helper so Markdown and Browser output remain
portable:

| StatusState | Fallback icon |
|-------------|---------------|
| `NotStarted` | `◻` |
| `Active` | `◽` |
| `Success` | `✓` |
| `Error` | `⤫` |
| `Warning` | `⚠` |
| `Info` | `ℹ` |
| `ToolUse` | `🔧` |
| `Subagent` | `🤖` |

Do not include the deprecated `Failure` variant in new CLI or documentation
surfaces. Persisted JSON compatibility for `"Failure"` remains a `StatusState`
concern, not a StatusBlock rendering concern.

Severity should be represented in three ways:

- Header paragraph `Style.color` uses the resolved severity color.
- Body `BlockQuote` `Style.border.color` uses the resolved severity color.
- Root and child nodes carry stable classes such as `status-block`,
  `status-block--error`, `status-block__header`, `status-block__body`, and
  `status-block__hint` so Browser output has styling hooks even before browser
  `Style` lowering is implemented.

### Body text extraction

The body is a `Vec<Prose>`. For this migration, render each `Prose`
optimistically, strip ANSI escape codes, and join items with `\n\n` before
placing the result in a `BlockQuote` paragraph. This matches the current
stacked body behavior while accepting the same styling loss already documented
for `BlockQuote::plain_text()`.

Header and hint text should go through the same prose-to-plain-text treatment
instead of inserting raw strings directly. That prevents bracketed prose tags
such as `<b>Shell Expansion Failed</b>` from leaking into Markdown or Browser
output.

### Layout and Style mapping

| StatusBlock input | Tree mapping |
|---|---|
| `layout` | Root `NodeAttrs::set_layout` |
| `severity.default_color()` | Header `Style.color`; body left border color |
| `border_color` override | Same style slots, overriding severity default |
| Default `border == "┃ "` | Body `Style.border` with `BorderWeight::Thick`, `BorderLineStyle::Solid`, and left-only `BorderSides` |
| Custom `border != "┃ "` | Do not encode in tree; keep a bespoke terminal fallback for this compatibility path |
| Header/body/hint prose styling | Flattened to text for the first migration |

The default border is not an accepted divergence: it can be represented by the
existing thick border primitive and should render as `┃` in the terminal tree
renderer. Arbitrary border prefixes are a separate compatibility issue.

### Implementation sketch

```rust
impl TreeRenderable for StatusBlock {
    fn render_tree(&self) -> RenderNode {
        use renderable::style::{
            Border, BorderLineStyle, BorderSides, BorderWeight, PerMode, Style,
        };
        use renderable::tree::RenderNode;

        let severity_color = self.resolved_border_color();
        let severity_class = format!("status-block--{}", self.severity_class());
        let mut children = Vec::new();

        if let Some(header_text) = &self.header {
            let header = self.render_header_fallback_text(header_text);
            let mut node = RenderNode::paragraph(vec![RenderNode::text(header)]);
            node.attrs.classes = vec!["status-block__header".into()];
            node.attrs.set_style(&Style {
                color: Some(TargetValue::universal(PerMode::universal(
                    severity_color.clone(),
                ))),
                ..Style::default()
            });
            children.push(node);
        }

        if !self.body.is_empty() {
            let body_text = self.body_plain_text();
            let mut node = RenderNode::block_quote(vec![RenderNode::paragraph(vec![
                RenderNode::text(body_text),
            ])]);
            node.attrs.classes = vec!["status-block__body".into()];
            node.attrs.set_style(&Style {
                border: Some(Border {
                    color: Some(TargetValue::universal(PerMode::universal(
                        severity_color.clone(),
                    ))),
                    weight: BorderWeight::Thick,
                    line_style: BorderLineStyle::Solid,
                    sides: BorderSides::Sides {
                        top: false,
                        right: false,
                        bottom: false,
                        left: true,
                    },
                    ..Border::default()
                }),
                ..Style::default()
            });
            children.push(node);
        }

        if let Some(hint_text) = &self.hint {
            let hint = self.prose_plain_text(hint_text);
            let mut node = RenderNode::paragraph(vec![RenderNode::text(hint)]);
            node.attrs.classes = vec!["status-block__hint".into()];
            children.push(node);
        }

        let mut root = RenderNode::root(children);
        root.attrs.classes = vec!["status-block".into(), severity_class];
        root.attrs.set_layout(&self.layout);
        root
    }
}
```

The exact helper names can differ, but the implementation should keep the
projection helper private and reusable so `TreeRenderable`, tests, and any
legacy compatibility hooks do not drift.

### TerminalRenderable strategy

Implement `TerminalRenderable` with a small branch:

- If `self.border == "┃ "`, delegate to `TreeComponent::new(self.clone())`.
- If `self.border != "┃ "`, keep the existing bespoke terminal renderer and
  document the branch as a compatibility fallback for arbitrary terminal
  prefixes.

This preserves public behavior for the existing `border(String)` API without
adding terminal-specific presentation strings to the canonical tree model.

### Parity test design

Add `biscuit-terminal/lib/tests/status_block_parity.rs`, following the pattern
in `render_tree_component_parity.rs`.

Semantic token parity between bespoke rendering and the tree route must cover:

1. Header-only content.
2. Body-only content.
3. Body with styled `Prose` content, accepting flattened styling.
4. Header plus body.
5. Header plus body plus hint.
6. Multiple body `Prose` items joined with a blank line.
7. Custom `border_color`, with SGR present in both paths on a color terminal.
8. Default layout, including right-margin-driven wrapping.
9. Explicit left and right margins.
10. Every non-deprecated `StatusState` and its fallback icon.

Accepted divergences:

- Prose styling is flattened in the tree projection.
- Header icon rendering is stable fallback Unicode in the tree path, while the
  legacy path may render Nerd Font icons on terminals that advertise them.
- A custom arbitrary border prefix uses the bespoke compatibility path and is
  not part of tree parity.

## Browser IR Implementation

StatusBlock should implement `BrowserRenderable` by delegating through the
existing `BrowserTreeComponent` adapter:

```rust
impl BrowserRenderable for StatusBlock {
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        BrowserTreeComponent::new(self.clone()).render_html_fragment()
    }

    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
        BrowserTreeComponent::new(self.clone()).render_html_page(page)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
```

Expected fragment shape for an error StatusBlock with all parts present:

```html
<div class="status-block status-block--error">
  <p class="status-block__header">⤫ Shell expansion failed</p>
  <blockquote class="status-block__body"><p>Missing closing brace</p></blockquote>
  <p class="status-block__hint">Check the template syntax and retry.</p>
</div>
```

The current browser renderer emits classes and layout-derived inline style, but
browser `Style` lowering is still listed as a known gap in
`layout-and-style.md`. Therefore color and border visuals are preserved in the
tree but may not be visible in browser output until that shared renderer work
lands.

Browser tests should cover:

1. Body-only output contains one `<blockquote>` and no header or hint class.
2. Header plus body preserves element order.
3. Header plus body plus hint emits all three classes.
4. Header-only output contains one paragraph.
5. `render_html_page(None)` returns a complete HTML page.
6. Every non-deprecated severity emits the expected fallback icon and root
   severity class.
7. Custom `border_color` is present in the tree style, with visual HTML
   enforcement deferred to browser `Style` lowering.

## Markdown IR Implementation

StatusBlock should implement `MarkdownRenderable` through
`render_markdown_node` on its tree projection:

```rust
impl MarkdownRenderable for StatusBlock {
    fn render_markdown(&self) -> String {
        let node = self.render_tree();
        render_markdown_node(&node, &MarkdownRenderOptions::default())
            .map(|rendered| rendered.output)
            .unwrap_or_default()
    }

    fn render_markdown_plus(&self) -> String {
        self.render_markdown()
    }
}
```

Markdown and MarkdownPlus intentionally match for this first migration. The
projection carries severity visuals in `Style`, and Markdown renderers ignore
`Style` by contract. The portable output is structural Markdown:

```markdown
⤫ Shell expansion failed

> Missing closing brace

Check syntax
```

Markdown tests should cover:

1. Body-only output is a single block quote.
2. Header plus body uses a paragraph, blank line, and block quote.
3. Header plus body plus hint has three sections separated by blank lines.
4. Header plus hint with no body has two paragraphs and no block quote.
5. Every non-deprecated severity emits the expected fallback icon.
6. Multiple body `Prose` items survive inside the block quote.
7. `render_markdown()` and `render_markdown_plus()` produce identical output.
8. `border_color` has no effect on Markdown output.
9. Prose tags in header/body/hint are flattened rather than emitted as raw
   angle-bracket text.

## `bt` CLI

Add a `bt status-block` subcommand. It should render through the component APIs
so the CLI exercises the same projection and adapters as library users.

### Current State

| Property | Status |
|----------|--------|
| CLI command exists | yes (`bt status-block`) |
| Render method | tree (terminal/Markdown/HTML) |
| Has `--md` switch | yes |
| Has `--html` switch | yes |
| Has `--example` switch | yes |
| Uses tree renderer | yes |

### Specification Design

1. Register `bt status-block` in `args.rs` and add
   `cli/src/commands/status_block.rs`.
2. Add `--md` and `--html` as mutually exclusive switches.
3. Add `--severity`, required unless `--example`, accepting
   `error`, `warning`, `info`, `success`, `active`, `not-started`,
   `tool-use`, and `subagent`.
4. Add optional `--header`, `--hint`, and `--border-color`.
5. Accept positional body text, with multiple values joined by spaces.
6. Add `--example`, using all parts of the component and printing the command.
7. Flatten shared `LayoutArgs`; apply left/right/top/bottom/alignment through
   the component layout helpers or direct `layout_mut()` updates.

Do not add a CLI flag for arbitrary `border(String)` in the first tree-backed
command. That API is terminal-only compatibility surface and would make the CLI
less portable across `--md` and `--html`.

### CLI argument sketch

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

### Example constants

```rust
const STATUS_BLOCK_EXAMPLE_CMD: &str =
    r#"bt status-block --severity error --header "<b>Shell Expansion Failed</b>" --hint "Check the template syntax and retry." "Missing closing brace in `${...}` directive.""#;
```

### Implementation notes

- For default terminal output, call `block.render(&term)` after constructing the
  component. The component's `TerminalRenderable` impl decides whether it can
  use the tree route or must use the compatibility fallback.
- For Markdown, call `block.render_markdown()`.
- For HTML, call `block.render_html_fragment().render()` or
  `block.render_html_page(None).render()` according to the CLI pattern used by
  neighboring commands.
- Do not wrap `block.render_tree()` in another `RenderNode::root`; the
  StatusBlock projection already returns a root node.
- `LayoutArgs` currently exposes `margin_left`, `margin_right`, `margin_top`,
  `margin_bottom`, and `alignment`. The implementation sketch must not assume
  old fields such as `layout.margin_left`.

## Feature Requests for Tree Rendering

### RT-STATUSBLOCK-001: Arbitrary terminal border prefix in render-tree Style

**DENIED**

this feature will not be added to the render-tree tree implementation. You
should try to still use the render-tree where practical and work around the
complexity but if the complexity is too great then you have permission to
create a bespoke IR implementation for this component.

Why: `StatusBlock::border(String)` is an arbitrary terminal prefix, not a
target-agnostic structural or style concept. The existing `Style::Border`
already represents the default `"┃ "` case with a thick left border. Promoting
custom prefix strings into the render tree would leak terminal presentation
into Browser and Markdown targets. Preserve custom prefixes with a narrow
bespoke terminal fallback instead.

### RT-STATUSBLOCK-002: Dedicated `NodeKind::Callout`

**DENIED**

this feature will not be added to the render-tree tree implementation. You
should try to still use the render-tree where practical and work around the
complexity but if the complexity is too great then you have permission to
create a bespoke IR implementation for this component.

Why: StatusBlock can be represented faithfully enough with existing `Root`,
`Paragraph`, and `BlockQuote` nodes plus `Style`, `Layout`, and classes. A
future callout node might be useful after multiple components prove a shared
severity/callout abstraction, but adding it now would be speculative and would
force every renderer to handle a new variant without a demonstrated need.

No StatusBlock-related render-tree feature request is approved. Nothing needs
to be added to `approved-render-tree-functionality.md`.

## Acceptance Criteria for Implementation

- [ ] `StatusBlock` implements `TreeRenderable` as a root containing optional
      header, body, and hint sections.
- [ ] The projected root carries layout directly in `NodeAttrs`.
- [ ] The projected root and children carry stable StatusBlock classes.
- [ ] The default `"┃ "` body border is represented as a thick left
      `Style::Border`.
- [ ] Arbitrary custom `border(String)` remains a documented terminal
      compatibility fallback and is not encoded in the tree.
- [ ] `TerminalRenderable` uses the tree route for target-agnostic
      StatusBlocks.
- [ ] `BrowserRenderable` delegates through `BrowserTreeComponent`.
- [ ] `MarkdownRenderable` delegates through `render_markdown_node`.
- [ ] `bt status-block` is registered with `--severity`, `--header`, `--hint`,
      `--border-color`, positional body text, `--md`, `--html`, and
      `--example`.
- [ ] `--md` and `--html` are mutually exclusive.
- [ ] No new CLI flag exposes arbitrary custom border prefixes.
- [ ] Parity tests validate semantic content, fallback icons, default thick
      border behavior, color output, layout, and custom-border fallback.
- [ ] Unit tests cover tree structure, classes, style, and layout.
- [ ] Unit tests cover Browser and Markdown implementations.
- [ ] CLI integration tests cover terminal, `--md`, `--html`, `--example`, all
      non-deprecated severities, and validation of mutually exclusive flags.
- [ ] Components table updated: StatusBlock Tree `no` -> `yes`, Browser `no` ->
      `yes`, Markdown `no` -> `yes`, IR State -> `tree default + bespoke
      compatibility fallback`, bt CLI -> `tree`.

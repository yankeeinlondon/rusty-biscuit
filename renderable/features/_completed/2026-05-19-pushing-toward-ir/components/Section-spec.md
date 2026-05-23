# Section — IR Rendering Design Specification

## Component Status

| Field      | Value                                                                        |
|------------|------------------------------------------------------------------------------|
| Name       | Section                                                                      |
| Kind       | Block                                                                        |
| Location   | `biscuit-terminal/lib/src/components/section.rs`                             |
| Terminal   | ✅ bespoke `TerminalRenderable`                                               |
| Browser    | ❌                                                                            |
| Markdown   | ❌                                                                            |
| Tree       | ⚠️ compatibility `render_tree_node()` exists; canonical `TreeRenderable` missing |
| IR State   | compatibility projection exists; old terminal renderer is still default      |
| bt CLI     | — (no CLI subcommand exists)                                                 |

Section renders a Markdown-style heading (h1-h6) followed by arbitrary block
content. The heading level controls both visual styling (bold for h1-h3,
italic for h4-h5, plain for h6) and the Markdown prefix (`#`, `##`, etc.).

A compatibility tree projection already exists via
`TerminalRenderable::render_tree_node()` at `section.rs:274`, producing
`NodeKind::Section { depth, heading, children }`. This is not the canonical
`renderable::tree::TreeRenderable` implementation required by
`TreeComponent`, `BrowserTreeComponent`, and future cross-target adapters.
Terminal, Browser, and Markdown tree renderers already handle
`NodeKind::Section` natively.

However, the default `TerminalRenderable::render()` still uses the bespoke
`render_content()` path. Section has no `TreeRenderable`,
`BrowserRenderable`, or `MarkdownRenderable` impl. There is no `bt section`
CLI subcommand.

---

## Design Steps

### Terminal IR Implementation

- The **Section** component does not currently have a IR based rendering solution
- This section will describe what is required to ensure that the **Section** component:
    - has an IR implementation
    - the IR implementation drives the TerminalRenderable contract
    - the IR implementation is what is used by the bt CLI (note if **Section** doesn't yet have bt CLI subcommand then it will be designed below in the bt CLI section)

#### Tree Projection Status

Section already has a working compatibility tree projection via
`render_tree_node()` at `section.rs:274`. The projection:

1. Maps `HeadingLevel` to `HeadingDepth` (1-6, validated by `HeadingDepth::new`).
2. Creates the heading as `vec![RenderNode::text(&self.title)]` — a single text
   node in the heading's phrasing content.
3. Projects each content item via `RenderableTerminalContent::to_tree_nodes()`,
   collecting the resulting nodes into the section body.
4. Seeds the component's `Layout` onto the projected node if non-default.

The native terminal tree renderer handles `NodeKind::Section` at
`render_tree/render.rs:335`:

- Applies the heading emphasis style (bold/italic/plain) via
  `heading_effective(depth)` and the shared `render_heading_line()` helper.
- `render_heading_line()` (line 732) produces the `# ` prefix, applies the
  declared `Style` through `style::apply_style()`, and wraps the heading text.
- Body children are rendered as blocks via `render_blocks()`.
- The heading and body are joined with a blank line separator
  (`{heading_output}\n\n{body}`).

This produces output semantically equivalent to the bespoke `render_content()`,
which also applies heading emphasis via `apply_style()`, emits the same `# `-
style prefix, and renders content items sequentially.

#### Canonical `TreeRenderable` Requirement

Before switching Terminal, Browser, Markdown, or CLI output to the tree path,
Section must implement `renderable::tree::TreeRenderable`. The existing
`TerminalRenderable::render_tree_node()` hook should remain as a compatibility
adapter and delegate to the same private projection helper as
`TreeRenderable::render_tree()`.

Required shape:

```rust
impl Section {
    fn project_tree(&self) -> RenderNode {
        // current render_tree_node body, except it returns RenderNode directly
        // because HeadingLevel::level() is guaranteed to be 1..=6.
    }
}

impl TreeRenderable for Section {
    fn render_tree(&self) -> RenderNode {
        self.project_tree()
    }
}

impl TerminalRenderable for Section {
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.project_tree())
    }
}
```

The projection helper must seed layout exactly once on the returned node's
`NodeAttrs`, matching the current `BlockQuote` pattern. Do not rely on
`TreeRenderable::tree_layout()` for this migration: the current adapters render
the `RenderNode` returned by `render_tree()` and do not apply that optional
hook. Callers must not also wrap the result in a `TreeComponent` carrying a
separate layout.

#### Switching the Default Render Path

The goal is to make the IR path the default for `TerminalRenderable::render()`
while **retaining the bespoke path** for parity testing.

The switch follows the same error policy as `TreeComponent`: render with
`RenderStrictness::Warn`, return a visible fallback string on structural
errors, and never panic in the infallible `TerminalRenderable` methods.

```rust
impl TerminalRenderable for Section {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = Terminal::new_optimistic(width);
        self.render_via_tree(&term)
    }

    fn render(&self, term: &Terminal) -> String {
        self.render_via_tree(term)
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn is_block_level(&self) -> bool {
        true
    }

    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.project_tree())
    }
}

impl Section {
    fn render_via_tree(&self, term: &Terminal) -> String {
        let node = self.project_tree();
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => format!("[render-tree error: {error}]"),
        }
    }

    /// Retained for parity testing. Renders via the pre-tree bespoke path.
    pub(crate) fn render_bespoke_for_comparison(&self, term: &Terminal) -> String {
        let width = term.width();
        let available = self.layout.available_width(width);
        let content = self.render_content(Some(term), available);
        self.layout.apply_layout(&content, width)
    }
}
```

#### Layout Mapping

Section's `Layout` already seeds onto the projected `RenderNode` via
`node.attrs.set_layout(&self.layout)` when non-default. The terminal tree
renderer's `render_with_layout` applies it during rendering. No additional
Layout parameters are needed.

Section is a block-level component; the Layout properties that matter are:
- `margin` — left/right/top/bottom spacing around the entire section
- `alignment` — horizontal position of the heading and body
- `word_wrap` — wrapping policy for content items

`max_width` is a Browser-only property and does not affect terminal rendering.

#### Style Considerations

Section's visual appearance is purely based on heading emphasis (bold/italic),
which is declared via `HeadingLevel::heading_style()` → `Style { emphasis,
..default() }`. The bespoke path already uses `apply_style()` for heading
rendering (line 195), and the tree renderer's `render_heading_line()` uses the
same `apply_style()` path (line 746). This means the style application is
already aligned between the two paths.

No additional `Style` fields (color, background, border, fill) are used by
Section — the component is structurally simple.

#### Parity Test Strategy

Existing parity tests are in `biscuit-terminal/lib/tests/section_parity.rs`.
The following critical test variants must be covered for the IR vs bespoke
comparison:

| Variant                                                       | Validates                                                                        |
|---------------------------------------------------------------|----------------------------------------------------------------------------------|
| h1 section with no content                                    | Heading prefix `# `, bold emphasis                                               |
| h2 section with string content                                | Heading prefix `## `, bold emphasis, body text present                           |
| h3 section with multiple content items                        | Heading prefix `### `, bold emphasis, both body items present and ordered        |
| h4 section                                                    | Heading prefix `#### `, italic emphasis                                          |
| h5 section                                                    | Heading prefix `##### `, italic emphasis                                         |
| h6 section                                                    | Heading prefix `###### `, no emphasis                                            |
| Section with Prose content item                               | Prose styling survives both paths                                                |
| Section with nested Component content                         | Component renders identically in both paths                                      |
| Section with Layout margins                                   | Left/right margins applied correctly in both paths                               |
| Section with Layout alignment                                 | Content alignment preserved in both paths                                        |
| Section at narrow terminal width (40 cols)                    | Content renders without overflow in both paths                                   |
| Section at wide terminal width (120 cols)                     | Content renders correctly in both paths                                          |
| Empty section (title only, no content)                        | Only heading rendered, no trailing blank lines                                   |
| Section with layout at all parity widths                      | Width matrix covers 40/60/80/100/120/160/200                                     |

Parity must use both the existing semantic tests in `section_parity.rs` and
the broader facet-based `render_comparison.rs` matrix. The existing
`section_parity.rs` tests only prove projection validity and semantic token
presence; they are not sufficient to flip the default render path by
themselves.

For the flip, update comparison coverage so it renders:

- the retained bespoke helper, `render_bespoke_for_comparison()`
- the default `TerminalRenderable::render()` tree path
- the direct `render_terminal_node(section.render_tree())` path

Parity is asserted on structural facets rather than only ANSI-stripped token
presence: exact bytes, visible text, indentation, blank-line positions,
maximum visible width, and SGR styling offsets. Known accepted divergences
must be documented in the existing `KNOWN_DRIFT` ledger with a verdict:

- **Blank line between heading and body**: The tree renderer emits
  `{heading}\n\n{body}` (double newline). The bespoke renderer emits
  `{heading}\n{content_items}` with single newlines between content items.
  After ANSI stripping, the heading and body tokens must be present in order,
  but the exact blank-line count may differ.
- **Layout application**: Both paths apply layout (margins, alignment), but
  the tree path does so through `render_with_layout` while the bespoke path
  uses `LayoutTerminalExt::apply_layout`. These should produce the same result
  but exercise different code paths.
- **Style offsets**: Heading styling is applied through the same style lowering
  helper, but the extra blank line can shift visible offsets. Record this only
  when the rendered heading itself is still styled correctly.

#### Feature Requests for Tree Rendering

No render-tree feature requests are needed for Section. No APPROVED or DENIED
stamp is required because this spec does not request a render-tree
implementation change. The existing tree renderer already has native support
for Section via `NodeKind::Section`:

- `render_heading_line()` reconstructs the heading with the correct prefix and
  emphasis style
- Body children render as blocks via `render_blocks()`
- Layout application via `render_with_layout`
- The `Style` system handles heading emphasis through `apply_style()`

#### Tree Renderer Fit Assessment

The existing tree renderer is an **excellent fit** for Section. Section is one
of the simplest structural components: a heading followed by block content. The
tree renderer already has native support that mirrors the bespoke rendering
logic exactly:

1. The compatibility tree projection already exists and passes structural tests
2. The terminal tree renderer handles `NodeKind::Section` natively
3. Heading emphasis is applied via the same `apply_style()` path in both
   bespoke and tree rendering
4. Content projection via `to_tree_nodes()` preserves text and component content

Section is an ideal candidate for tree-first rendering because:
- It maps directly to a well-supported `NodeKind` variant
- Its visual appearance is entirely driven by `Style.emphasis`, which the tree
  renderer already handles
- There are no widget-specific hints or custom rendering hooks needed
- The component has no visual complexity beyond heading styling and content layout
- The only missing component work is canonical trait adoption and parity
  hardening, not new render-tree vocabulary

`will_use_tree_renderer`: **true** — the existing tree renderer handles
Section's needs without any feature additions.

`will_use_tree_renderer_with_features`: **false** — no render-tree features
are requested or required.

---

### Browser IR Implementation

- in this section we will provide a design specification for the **Section** component's implementation of the BrowserRenderable trait

Section does not currently have a bespoke browser rendering implementation.
Once Section implements `TreeRenderable`, browser rendering can leverage the
existing browser tree renderer.

#### Browser Rendering Design

The browser tree renderer (`renderable/src/tree/render/browser.rs`) already
handles `NodeKind::Section` natively via `render_section()`. It produces:

```html
<section>
  <h2>Getting Started</h2>
  <p>Welcome to the tutorial.</p>
  <p>Let's begin with installation.</p>
</section>
```

The `render_section()` implementation:
1. Creates a `<section>` element with node attributes (layout → CSS via
   `layout_to_css`).
2. Renders the heading as `<h1>`-`<h6>` based on `HeadingDepth`, with inline
   children.
3. Renders body children as blocks within the section.

No changes are needed in the browser tree renderer — Section maps perfectly to
the existing `NodeKind::Section` handling.

**BrowserTreeComponent adapter:**

Call sites that own a `Section` can render through the adapter after
`TreeRenderable` is implemented:

```rust
use biscuit_terminal::render_tree::BrowserTreeComponent;
use renderable::browser::BrowserRenderable;

let section = Section::new(HeadingLevel::h2, "Getting Started");
let component = BrowserTreeComponent::new(section);
let fragment = component.render_html_fragment();
let html = fragment.render();
```

Section itself should implement `BrowserRenderable` directly by projecting its
tree and calling the browser renderer. The direct impl cannot use
`BrowserTreeComponent::new(self)` because `render_html_fragment(&self)` only
has a shared borrow and the adapter owns its inner component.

```rust
impl BrowserRenderable for Section {
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        render_browser_node(&self.render_tree(), &BrowserRenderOptions::default())
            .expect("browser render of Section should succeed")
            .output
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
```

The direct impl is the public component surface. `BrowserTreeComponent` remains
useful in generic call sites and tests that already traffic in owned
`TreeRenderable` values.

#### Layout to CSS Mapping

Section's `Layout` maps to CSS via the existing `layout_to_css` lowering:

- Margins → `margin-*` properties on the `<section>` element
- Alignment → `auto` margins when `max_width` is present
- `max_width` → `max-width` CSS property
- `word_wrap` → `white-space` / `overflow-wrap` CSS properties

No additional CSS mapping is needed.

#### Key Test Variants

| Variant                                    | Asserts                                                                      |
|--------------------------------------------|------------------------------------------------------------------------------|
| h1 section, no content                     | `<section><h1>Title</h1></section>`                                          |
| h2 section with string content             | `<section><h2>Title</h2><p>content</p></section>`                            |
| h3 section with multiple content items     | Multiple `<p>` elements in body                                              |
| h4 section                                 | `<h4>` heading tag                                                           |
| h5 section                                 | `<h5>` heading tag                                                           |
| h6 section                                 | `<h6>` heading tag                                                           |
| Section with Layout margins                | `<section>` has `margin-left` / `margin-right` inline style                  |
| Section with Layout alignment + max-width  | `<section>` has `max-width` and auto horizontal margins; no `text-align` expectation |
| Empty section                              | Only `<section><hN>Title</hN></section>`, no body children                   |
| Section with nested component content      | Nested component renders as expected within `<section>`                      |

---

### Markdown IR Implementation

#### Markdown vs MarkdownPlus for Section

Section is a structural component whose content is inherently representable in
pure Markdown: a heading line (`## Title`) followed by body content. This means
**Markdown and MarkdownPlus produce identical output** for Section — there is
no inline HTML, no colors, and no visual styling that Markdown cannot represent.

The heading prefix (`#`, `##`, etc.) and body paragraphs are pure Markdown
syntax. No divergence between Markdown and MarkdownPlus is expected.

**Divergence examples:**

For `Section::new(HeadingLevel::h2, "Getting Started").push("Welcome.")`:

- **Markdown**: `## Getting Started\n\nWelcome.`
- **MarkdownPlus**: `## Getting Started\n\nWelcome.` (identical)

Even if Section's heading carries a `Style` with emphasis, the Markdown renderer
ignores `Style` by design (locked by test), so both outputs remain identical.

If Section's body content includes Prose with colors (e.g.,
`Prose::new("<red>Error</red>")`), the Markdown renderer would strip the color
and produce `Error`, while MarkdownPlus would also produce `Error` (since
Section itself doesn't add color — the divergence would come from the child
Prose, not Section). For Section itself, the two targets never diverge.

#### Markdown Rendering Design

The Markdown tree renderer (`renderable/src/tree/render/markdown.rs`) already
handles `NodeKind::Section` at line 181:

```rust
NodeKind::Section { depth, heading, children } => {
    let hashes = "#".repeat(usize::from(depth.get()));
    let heading_line = format!("{hashes} {}", self.render_inline(heading)?);
    let body = self.render_blocks(children)?;
    if body.is_empty() {
        Ok(heading_line)
    } else {
        Ok(format!("{heading_line}\n\n{body}"))
    }
}
```

This produces exactly the Markdown output Section needs — a heading line with
`#`-prefix, followed by body content separated by blank lines. No changes are
needed in the Markdown tree renderer.

Section implements `MarkdownRenderable` by projecting its tree and calling the
Markdown renderer:

```rust
impl MarkdownRenderable for Section {
    fn render_markdown(&self) -> String {
        render_markdown_node(&self.render_tree(), &MarkdownRenderOptions::default())
            .map(|r| r.output)
            .unwrap_or_else(|_| self.title.clone())
    }

    fn render_markdown_plus(&self) -> String {
        let opts = MarkdownRenderOptions {
            dialect: MarkdownDialect::MarkdownPlus,
            ..MarkdownRenderOptions::default()
        };
        render_markdown_node(&self.render_tree(), &opts)
            .map(|r| r.output)
            .unwrap_or_else(|_| self.title.clone())
    }
}
```

Layout is ignored by the Markdown renderer (by design — locked by test).

#### Key Test Variants

| Variant                                    | Asserts                                                                      |
|--------------------------------------------|------------------------------------------------------------------------------|
| h1 section, no content — Markdown          | Output is `"# Title"`                                                        |
| h2 section with content — Markdown         | Output is `"## Title\n\ncontent"`                                            |
| h3 section with multiple items — Markdown  | All body items present, separated by blank lines                             |
| h4 section — Markdown                      | Output is `"#### Title"`                                                     |
| h6 section — Markdown                      | Output is `"###### Title"`                                                   |
| Markdown equals MarkdownPlus               | Both methods produce identical output for all heading levels                  |
| Section with Layout — Markdown             | Layout has no effect on Markdown output (regression test)                     |
| Section with Layout — MarkdownPlus         | Layout has no effect on MarkdownPlus output (regression test)                 |
| Section with Prose content — Markdown      | Prose text content present (styling stripped by Prose's own IR)               |
| Empty section — Markdown                   | Output is just the heading line with no trailing blank lines                  |

---

### `bt` CLI

- this specification will ensure that the **Section** component:
    - has a 'bt' CLI subcommand for rendering this component
    - that the '--md' and '--html' CLI switches are available to render to Markdown and HTML targets respectively (the default render is always for the Terminal)
    - that the '--example' CLI switch is in place to provide a thoughtful example of how this command should be used with the CLI (see other working examples for a template)

#### Current State

| Aspect              | Status                                                                |
|---------------------|-----------------------------------------------------------------------|
| CLI command exists  | No — no `bt section` command exists                                   |
| Render method       | N/A (no CLI command)                                                  |
| Has `--md` switch   | No                                                                    |
| Has `--html` switch | No                                                                    |
| Has `--example`     | No                                                                    |

There is no `bt section` command. Section is the only block-level component
with a tree projection that has no CLI surface.

#### Specification Design

Add a new `bt section` subcommand, following the pattern established by
`bt prose` and `bt block`.

**Args:**

| Flag                          | Type           | Description                                                       |
|-------------------------------|----------------|-------------------------------------------------------------------|
| `TITLE`                       | `Option<String>` | Section heading text (required unless `--example`)              |
| `--level` / `-l`              | `Option<u8>`   | Heading level 1-6 (default: 2)                                    |
| `--content` / `-c`            | `Vec<String>`  | Body content items (strings)                                       |
| `--example` / `-e`            | `bool`         | Render example and show command                                   |
| `--html`                      | `bool`         | Render to HTML fragment (conflicts with `--md`, `--md-plus`)      |
| `--md`                        | `bool`         | Render to portable Markdown (conflicts with `--html`, `--md-plus`)|
| `--md-plus`                   | `bool`         | Render to MarkdownPlus (conflicts with `--html`, `--md`)          |
| `#[command(flatten)] layout`  | `LayoutArgs`   | Shared margin/alignment arguments; currently left/right/top/bottom/alignment only |

**Implementation in `commands/section.rs`:**

```rust
#[derive(ClapArgs, Debug, Clone)]
pub struct SectionArgs {
    /// Render an example and show the command used
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Section heading text
    #[arg(value_name = "TITLE", required_unless_present = "example")]
    pub title: Option<String>,

    /// Heading level (1-6, default: 2)
    #[arg(long, short = 'l', default_value_t = 2)]
    pub level: u8,

    /// Body content items
    #[arg(long, short = 'c')]
    pub content: Vec<String>,

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with_all = ["md", "md_plus"])]
    pub html: bool,

    /// Render to portable Markdown instead of the terminal.
    #[arg(long, conflicts_with_all = ["html", "md_plus"])]
    pub md: bool,

    /// Render to MarkdownPlus instead of the terminal.
    #[arg(long = "md-plus", conflicts_with_all = ["html", "md"])]
    pub md_plus: bool,

    #[command(flatten)]
    pub layout: LayoutArgs,
}
```

The `run()` method:

1. Builds a `Section` from flags (title, level, content items).
2. Applies layout flags (margins, alignment).
3. **Terminal** (default): Project tree → `render_terminal_node()`.
4. **HTML** (`--html`): Wrap in `BrowserTreeComponent` → `render_html_fragment()`.
5. **Markdown** (`--md`): Project tree → `render_markdown_node()`.
6. **MarkdownPlus** (`--md-plus`): Project tree → `render_markdown_node()` with
   MarkdownPlus mode.

```rust
impl Run for SectionArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let title = if self.example {
            SECTION_EXAMPLE_TITLE.to_string()
        } else {
            self.title.ok_or_else(|| {
                color_eyre::eyre::eyre!("Title is required. Usage: bt section \"My Title\"")
            })?
        };

        let level = match self.level {
            1..=6 => heading_level_from_u8(self.level),
            _ => return Err(color_eyre::eyre::eyre!(
                "Heading level must be 1-6, got {}", self.level
            )),
        };

        let content = if self.example {
            vec![SECTION_EXAMPLE_BODY.to_string()]
        } else {
            self.content
        };

        let mut section = Section::new(level, &title);
        for item in &content {
            section.push(item.as_str());
        }

        apply_section_layout(&mut section, &self.layout);

        // Cross-target output
        if self.html {
            let component = BrowserTreeComponent::new(section);
            println!("{}", component.render_html_fragment().render());
            return Ok(());
        }
        if self.md {
            let rendered = render_markdown_node(&section.render_tree(), &MarkdownRenderOptions::default())
                .map_err(|e| color_eyre::eyre::eyre!("markdown render failed: {e}"))?;
            println!("{}", rendered.output);
            return Ok(());
        }
        if self.md_plus {
            let opts = MarkdownRenderOptions {
                dialect: MarkdownDialect::MarkdownPlus,
                ..MarkdownRenderOptions::default()
            };
            let rendered = render_markdown_node(&section.render_tree(), &opts)
                .map_err(|e| color_eyre::eyre::eyre!("markdown render failed: {e}"))?;
            println!("{}", rendered.output);
            return Ok(());
        }

        // Terminal (default)
        let term = detect_terminal_honoring_force_color();
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        let rendered = render_terminal_node(&section.render_tree(), &opts)
            .map_err(|e| color_eyre::eyre::eyre!("render failed: {e}"))?;

        println!("{}", rendered.output);

        if self.example {
            print_example_command(SECTION_EXAMPLE_CMD);
        }
        Ok(())
    }
}
```

`apply_section_layout()` should map all fields in `LayoutArgs` onto
`section.layout_mut()` before tree projection, including `margin_top` and
`margin_bottom`. Do not also call `emit_vertical_margins()` for the same output;
the tree renderer already applies vertical margins from node `Layout`, and
using both would double the top/bottom spacing.

**Example command:**

```rust
const SECTION_EXAMPLE_TITLE: &str = "Deployment Guide";
const SECTION_EXAMPLE_BODY: &str = "Run `bt section --level 2 \"My Title\" --content \"Body text\"` to render a section.";
const SECTION_EXAMPLE_CMD: &str =
    r#"bt section "Deployment Guide" --level 2 --content "Follow these steps to deploy." --content "Verify the build passes.""#;
```

**Registration in `args.rs`:**

Add `Section(section::SectionArgs)` to the `Command` enum with an appropriate
`display_order`, and add `pub mod section;` to `commands/mod.rs`.

---

## Acceptance Criteria Summary

- [ ] `Section`'s `TerminalRenderable::render()` delegates to the tree path by default
- [ ] Section implements canonical `TreeRenderable`; `render_tree_node()` delegates to the same projection helper
- [ ] Bespoke render path retained as `render_bespoke_for_comparison()` or equivalent for parity testing
- [ ] `BrowserRenderable` achieved — Section renders to `<section>` with appropriate `<h1>`-`<h6>` heading
- [ ] `MarkdownRenderable` implemented on Section — both Markdown and MarkdownPlus output `## Title\n\nbody` format
- [ ] `bt section` CLI subcommand exists with `--html`, `--md`, `--md-plus`, `--level`, `--content`, and `--example` switches
- [ ] `bt section --example` renders example output with the full CLI command displayed
- [ ] Parity tests (bespoke vs tree/default tree/direct tree) cover all heading levels, content variants, and layout configurations
- [ ] `render_comparison.rs` `KNOWN_DRIFT` ledger documents accepted divergences between bespoke and tree paths
- [ ] Existing parity tests in `section_parity.rs` continue to pass with the new default render path

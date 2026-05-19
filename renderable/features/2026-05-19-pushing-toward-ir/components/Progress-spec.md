# Progress — IR Rendering Design Specification

## Component Status

| Field      | Value                                                                                 |
|------------|---------------------------------------------------------------------------------------|
| Name       | Progress                                                                              |
| Kind       | Block (rendered as inline-block; visually a single line)                              |
| Location   | `biscuit-terminal/lib/src/components/progress.rs`                                     |
| Terminal   | ✅ bespoke `TerminalRenderable`                                                        |
| Browser    | ❌                                                                                     |
| Markdown   | ❌                                                                                     |
| Tree       | ✅ `render_tree_node()` exists (projects to `NodeKind::Paragraph` with `ProgressHints`)|
| IR State   | both avail, old renders                                                               |
| bt CLI     | tree (already routes through `render_terminal_node`)                                  |

Progress renders a horizontal progress bar showing completion percentage with
configurable width, fill/empty characters, bracket glyphs, and slot colors
(`filled_color`, `empty_color`, `bracket_color`). The bar is a single visual
line: `[████████░░░░░░░░░░░░]  40%`, optionally preceded by a label.

A tree projection already exists via `render_tree_node()`, producing a
`NodeKind::Paragraph` carrying `ProgressHints` on `NodeAttrs`. The paragraph's
visible text is `"{label} {percentage}%"` (or `"{percentage}%"` without a label)
so renderers without progress hint support degrade gracefully to plain text.
The bt CLI already uses the tree renderer path.

However, the default `TerminalRenderable::render()` still uses the bespoke
`render_bar()` path. The browser and markdown tree renderers do not handle
`ProgressHints` — they render the paragraph as-is (`<p>` / plain text).

---

## Design Steps

### Terminal IR Implementation

- The **Progress** component does not currently have a IR based rendering solution
- This section will describe what is required to ensure that the **Progress** component:
    - has an IR implementation
    - the IR implementation drives the TerminalRenderable contract
    - the IR implementation is what is used by the bt CLI (note if **Progress** doesn't yet have bt CLI subcommand then it will be designed below in the bt CLI section)

#### Tree Projection Status

Progress already has a working tree projection via `render_tree_node()` at
`progress.rs:268`. The projection:

1. Computes the percentage string (`"{label} {pct}%"` or `"{pct}%"`).
2. Creates `RenderNode::paragraph(vec![RenderNode::text(visible)])`.
3. Seeds `ProgressHints` onto the node's `NodeAttrs` with all bar parameters
   (`value`, `bar_width`, `fill_char`, `empty_char`, `left_bracket`,
   `right_bracket`, `filled_color`, `empty_color`, `bracket_color`).
4. Seeds the component's `Layout` if non-default.

The native terminal tree renderer already handles `ProgressHints` at
`render_tree::render.rs:357`: when a `NodeKind::Paragraph` carries progress
hints, it calls `render_progress_bar()` (line 1202), which reconstructs the
full bar from the hints — fill/empty segments, bracket glyphs, color
application via `paint_fg`, and label extraction. This produces output
identical to the bespoke `Progress::render_bar()`.

#### Switching the Default Render Path

The goal is to make the IR path the default for `TerminalRenderable::render()`
while **retaining the bespoke path** for parity testing.

The switch follows the same pattern as OrderedList/UnorderedList: Progress's
`render()` and `render_optimistic()` delegate to a `render_via_tree()` helper,
and the old bespoke logic is preserved as `render_bespoke()`.

**Implementation approach:**

```rust
impl TerminalRenderable for Progress {
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

    fn render_tree_node(&self) -> Option<RenderNode> {
        // existing projection unchanged
    }
}

impl Progress {
    fn render_via_tree(&self, term: &Terminal) -> String {
        let node = self.render_tree_node().expect("Progress always projects");
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => format!("[render-tree error: {error}]"),
        }
    }

    /// Retained for parity testing. Renders via the pre-tree bespoke path.
    fn render_bespoke(&self, term: &Terminal) -> String {
        let width = term.width();
        let bar_content = self.render_bar(term.color_depth);
        self.layout.apply_block_layout(&bar_content, width)
    }
}
```

#### Layout Mapping

Progress's `Layout` already seeds onto the projected `RenderNode` via
`node.attrs.set_layout(&self.layout)` (line 288). The terminal tree renderer's
`render_with_layout` applies it during rendering. No additional Layout
parameters are needed.

Progress is a single-line component; the only Layout properties that matter are
`margin` (left/right/top/bottom spacing) and `alignment` (horizontal position).
`max_width` is a Browser-only property and `word_wrap` does not apply to a
progress bar (it is inherently non-wrapping).

#### Style Considerations

Progress uses a typed `ProgressStyle` struct (not the shared `renderable::style::Style`)
for its visual appearance. The `ProgressStyle` carries glyph characters and slot
colors, which are encoded into `ProgressHints` on the projected node. The
terminal tree renderer reads these hints directly in `render_progress_bar()`,
bypassing the generic `Style` system.

This is the correct approach: progress bar glyphs and track colors are
widget-specific concepts that don't map naturally to `Style`'s box-painting
model (border, fill, background). `ProgressStyle` is already a Spec B D5
compliant typed component style struct.

#### Parity Test Strategy

Critical test variants for the IR vs bespoke comparison:

| Variant                                                       | Validates                                                                        |
|---------------------------------------------------------------|----------------------------------------------------------------------------------|
| Zero progress (`0.0`)                                         | No fill chars, all empty chars, `"  0%"`                                         |
| Full progress (`1.0`)                                         | All fill chars, no empty chars, `"100%"`                                         |
| Half progress (`0.5`)                                         | Equal fill and empty counts, `"50%"`                                             |
| Progress with label                                           | Label precedes bar in both paths                                                 |
| Custom bar width (e.g., 10)                                   | 5 fill + 5 empty at 50%                                                          |
| Custom glyphs (fill='#', empty='-', brackets='(', ')')        | All custom glyphs present in output                                              |
| Custom fill color (green)                                     | SGR escape `\x1b[32m` wraps fill chars in both paths                            |
| Custom bracket color (cyan)                                   | SGR escape `\x1b[36m` wraps brackets in both paths                              |
| All three slot colors set                                     | All three color SGR sequences present                                            |
| Color depth degradation (truecolor → basic → none)            | RGB color degrades to basic, then strips entirely at `ColorDepth::None`          |
| No colors (default)                                           | No SGR escapes in output                                                         |
| Value clamped above 1.0                                       | Both paths produce `100%`                                                        |
| Value clamped below 0.0                                       | Both paths produce `0%`                                                          |
| Percentage alignment (`0%`, `75%`, `100%`)                    | Right-aligned format `{percentage:3}%` preserved                                 |
| Left margin applied                                           | Layout margin prefixes the line with spaces                                      |
| Right margin applied                                          | Available width is narrowed                                                      |
| Center alignment                                              | Bar is centered within the available width                                       |
| Small terminal width (bar wider than terminal)                | Behavior is defined and consistent                                               |
| `ProgressStyle` serde roundtrip                               | JSON serialization preserves all fields                                          |

Parity is asserted on **ANSI-stripped content equality** (not byte-identical
output), following the BlockQuote parity discipline. The content semantics —
fill count, empty count, label text, percentage — must be identical. Known
accepted divergences should be documented in a `KNOWN_DRIFT` ledger:

- **SGR sequence ordering**: The bespoke path emits color escapes in
  `render_bar()`'s direct `paint_fg` calls. The tree path emits them in
  `render_progress_bar()` which follows the same logic but may produce
  slightly different escape ordering. After ANSI stripping, content must
  be identical.
- **Layout application**: Both paths apply layout (margins, alignment), but
  the tree path does so through `render_with_layout` while the bespoke path
  uses `LayoutTerminalExt::apply_block_layout`. These should produce the
  same result but exercise different code paths.

#### Feature Requests for Tree Rendering

No feature requests are needed. The existing tree renderer already has native
support for progress bars via `ProgressHints` on `NodeKind::Paragraph`:

- `render_progress_bar()` reconstructs the full bar from hints
- Color slot degradation through `paint_fg` matches the bespoke path
- Label extraction from paragraph text
- Layout application via `render_with_layout`

#### Tree Renderer Fit Assessment

The existing tree renderer is an **excellent fit** for Progress. The component's
entire rendering logic — fill/empty glyph painting, bracket rendering, slot
color application with depth degradation, percentage formatting, and label
prefix — has a corresponding native implementation in `render_progress_bar()`.

The `ProgressHints` mechanism is a clean, well-designed bridge: all bar
parameters are carried on the node, and the renderer reconstructs the visual
bar from those parameters. The paragraph's visible text provides a graceful
degradation path for renderers that don't understand progress hints.

Progress is one of the simplest components to migrate because:
1. The tree projection already exists and is well-tested
2. The terminal tree renderer already has native progress bar support
3. The bt CLI already uses the tree path
4. The component is a single-line widget with no nesting or complex layout

`will_use_tree_renderer`: **true** — the existing tree renderer handles
Progress's needs without any feature additions.

`will_use_tree_renderer_with_features`: **true** — no features requested, so
this is the same as above.

---

### Browser IR Implementation

- In this section we will provide a design specification for the **Progress** component's implementation of the BrowserRenderable trait

Progress does not currently have a bespoke browser rendering implementation.
Since Terminal IR is designed first and Progress already projects to a
`NodeKind::Paragraph` with `ProgressHints`, the browser rendering must handle
the progress hints to produce a visual progress bar in HTML.

#### Browser Rendering Design

Unlike terminal rendering where `ProgressHints` are handled by the terminal tree
renderer, the browser tree renderer (`renderable/src/tree/render/browser.rs`)
does **not** handle `ProgressHints`. It treats a `NodeKind::Paragraph` as `<p>`
with inline children, ignoring the progress hints entirely. The visible text
degrades to `"{label} {percentage}%"` — functional but not visual.

There are two design options:

**Option A: Handle `ProgressHints` in the browser tree renderer.** Add a check
in the browser renderer's `NodeKind::Paragraph` branch (line 205): when
`progress_hints()` is present, emit a `<div class="progress">` with inline
styles instead of `<p>`. This mirrors what the terminal renderer does.

**Option B: Implement `BrowserRenderable` directly on `Progress`.** Write a
bespoke browser emitter that produces the HTML fragment, bypassing the tree.

**Recommendation: Option A** — extending the browser tree renderer is the
correct architectural choice because:

1. It follows the established pattern (terminal renderer already handles
   `ProgressHints`).
2. It ensures the tree is the single source of truth — any component that
   projects `ProgressHints` gets browser support automatically.
3. It avoids duplicating the bar reconstruction logic in a second location.

**Implementation for browser tree renderer:**

In `renderable/src/tree/render/browser.rs`, the `NodeKind::Paragraph` arm
checks for progress hints:

```rust
NodeKind::Paragraph { children } => {
    if let Some(hints) = node.attrs.progress_hints() {
        self.render_progress(node, &hints, children)
    } else {
        self.block(BlockTag::P, &node.attrs, children)
    }
}
```

`render_progress` produces an HTML progress bar:

```html
<div class="progress" style="...">
  <span class="progress-label">Loading</span>
  <div class="progress-track" style="width: 28ch">
    <div class="progress-filled" style="width: 75%; background-color: green;"></div>
  </div>
  <span class="progress-percentage">75%</span>
</div>
```

CSS properties:
- The outer `div` carries Layout as inline `style` (via `layout_to_css`).
- The track width is `bar_width` in `ch` units.
- The filled portion width is `value * 100%`.
- Colors are rendered as CSS color values (`rgb(r,g,b)` or named colors).
- The label and percentage are text nodes.
- Brackets are not rendered in the browser — the CSS visual is a modern
  progress bar, not a terminal character-art bar.

**BrowserTreeComponent adapter:**

Progress gains `BrowserRenderable` by wrapping in the adapter:

```rust
use biscuit_terminal::render_tree::BrowserTreeComponent;
use renderable::browser::BrowserRenderable;

let bar = Progress::new(0.75).with_label("Loading");
let component = BrowserTreeComponent::new(bar);
let fragment = component.render_html_fragment();
let html = fragment.render();
```

#### Layout to CSS Mapping

Progress's `Layout` maps to CSS via the existing `layout_to_css` lowering in
`renderable/src/tree/render/browser.rs`:

- Margins → `margin-*` properties on the outer `<div>`
- Alignment → `text-align` when `max_width` is present
- `max_width` → `max-width` CSS property

No additional CSS mapping is needed beyond what the tree renderer already
provides.

#### Key Test Variants

| Variant                                    | Asserts                                                                      |
|--------------------------------------------|------------------------------------------------------------------------------|
| Zero progress (`0.0`)                      | `width: 0%` on filled div, `"0%"` text                                      |
| Full progress (`1.0`)                      | `width: 100%` on filled div, `"100%"` text                                  |
| Half progress (`0.5`)                      | `width: 50%` on filled div, `"50%"` text                                    |
| With label                                 | HTML contains `<span class="progress-label">Loading</span>`                 |
| Without label                              | No label span in output                                                      |
| Custom bar width                           | Track width matches (e.g., `width: 28ch`)                                    |
| Fill color                                 | `background-color` on filled div matches the color                           |
| Empty color                                | `background-color` on track div matches the color                            |
| No colors (default)                        | No `background-color` on filled or track div                                 |
| Layout with margins                        | Outer `div` has `margin-left` / `margin-right` CSS                           |
| Layout with alignment and max-width        | Outer `div` has `text-align` and `max-width` CSS                             |
| Glyph characters are not rendered          | No fill/empty char glyphs in HTML (visual is CSS-based)                      |
| Bracket characters are not rendered        | No `[` `]` characters in HTML                                                |

---

### Markdown IR Implementation

#### Markdown vs MarkdownPlus for Progress

Progress is a visual widget — its rendering is based on character glyphs,
fill/empty ratios, and colored segments. This creates a clear separation point
between Markdown and MarkdownPlus:

- **Markdown**: The progress bar's semantic content is the label, percentage,
  and value. Markdown cannot represent the visual bar, colors, or glyphs.
  Output degrades to plain text: `"Loading 75%"` (with label) or `"75%"`.
- **MarkdownPlus**: Inline HTML can represent the bar as a styled `<div>`.
  The MarkdownPlus output uses an inline HTML `<div>` with a CSS-styled
  progress bar, preserving the visual representation.

For a progress bar without colors, MarkdownPlus could render a text-based bar
using Markdown-compatible characters: `Loading [████████████████░░░░░░░░] 75%`.
However, since this is a text-art representation and not true Markdown syntax,
the MarkdownPlus output should use inline HTML for fidelity.

**Divergence examples:**

For `Progress::new(0.75).with_label("Loading")` with green fill color:

- **Markdown**: `Loading 75%`
- **MarkdownPlus**: `<div class="progress">Loading <div style="display:inline-block;width:28ch"><div style="width:75%;background-color:green">&nbsp;</div></div> 75%</div>`

For `Progress::new(0.5)` with no colors:

- **Markdown**: `50%`
- **MarkdownPlus**: `50%` (identical — no colors to preserve, no bar to render)

The key insight: **when there are no colors, both Markdown and MarkdownPlus
should produce the same output** (just the label and percentage). When there
are colors or the visual bar is semantically important, MarkdownPlus uses
inline HTML.

#### Markdown Rendering Design

The Markdown tree renderer (`renderable/src/tree/render/markdown.rs`) currently
does not handle `ProgressHints`. It renders a `NodeKind::Paragraph` as inline
text — which correctly produces `"Loading 75%"` for Markdown output.

This means:

1. **Markdown**: The default behavior (ignoring `ProgressHints` and rendering
   paragraph text) is already correct. No changes needed in the markdown tree
   renderer for plain Markdown.

2. **MarkdownPlus**: The renderer needs to detect `ProgressHints` and produce
   inline HTML when the bar has visual elements (colors, or explicit visual
   rendering requested).

**Implementation approach for MarkdownPlus:**

Add a check in the markdown renderer's `NodeKind::Paragraph` branch:

```rust
NodeKind::Paragraph { children } => {
    if self.mode == MarkdownMode::Plus {
        if let Some(hints) = node.attrs.progress_hints() {
            return self.render_progress_markdown_plus(&hints, children);
        }
    }
    self.render_inline(children)
}
```

`render_progress_markdown_plus` produces inline HTML only when there are colors:

```rust
fn render_progress_markdown_plus(&self, hints: &ProgressHints, children: &[RenderNode]) -> Result<String, RenderError> {
    if hints.filled_color.is_none() && hints.empty_color.is_none() && hints.bracket_color.is_none() {
        // No colors — degrade to plain text (same as Markdown)
        return self.render_inline(children);
    }
    // Produce inline HTML progress bar
    let percentage = (hints.value * 100.0).round() as u32;
    let label = /* extract from children */;
    let mut html = String::new();
    if let Some(label) = label {
        html.push_str(&format!("{label} "));
    }
    html.push_str(&format!(
        "<span style=\"display:inline-block;width:{}ch\">",
        hints.bar_width
    ));
    html.push_str(&format!(
        "<span style=\"display:inline-block;width:{}%;background-color:{}\">&nbsp;</span>",
        percentage,
        css_color_for(hints.filled_color)
    ));
    html.push_str("</span>");
    html.push_str(&format!(" {percentage}%"));
    Ok(html)
}
```

Progress can implement `MarkdownRenderable` by projecting its tree and calling
the markdown renderer:

```rust
impl MarkdownRenderable for Progress {
    fn render_markdown(&self) -> String {
        let node = self.render_tree_node().unwrap();
        render_markdown_node(&node, &MarkdownRenderOptions::default())
            .map(|r| r.output)
            .unwrap_or_else(|_| format!("{}%", (self.value * 100.0).round() as u32))
    }

    fn render_markdown_plus(&self) -> String {
        let node = self.render_tree_node().unwrap();
        render_markdown_node(&node, &MarkdownRenderOptions::default_plus())
            .map(|r| r.output)
            .unwrap_or_else(|_| format!("{}%", (self.value * 100.0).round() as u32))
    }
}
```

Layout is ignored by the Markdown renderer (by design — locked by test).

#### Key Test Variants

| Variant                                    | Asserts                                                                      |
|--------------------------------------------|------------------------------------------------------------------------------|
| Zero progress — Markdown                   | Output is `"0%"`                                                             |
| Zero progress — MarkdownPlus               | Output is `"0%"` (identical when no colors)                                  |
| Half progress — Markdown                   | Output is `"50%"`                                                            |
| Half progress with label — Markdown        | Output is `"Loading 50%"`                                                    |
| Full progress — Markdown                   | Output is `"100%"`                                                           |
| With colors — Markdown                     | Output is `"50%"` (colors dropped)                                           |
| With colors — MarkdownPlus                 | Output contains inline HTML `<span style=...>`                               |
| Without colors — Markdown                  | Output is `"50%"`                                                            |
| Without colors — MarkdownPlus              | Output is `"50%"` (identical to Markdown)                                    |
| With label and colors — Markdown           | Output is `"Loading 50%"`                                                    |
| With label and colors — MarkdownPlus       | Output is `"Loading <span ...>...</span> 50%"`                                |
| Markdown equals MarkdownPlus (no colors)   | Both methods produce identical output                                         |
| Progress with Layout — Markdown            | Layout has no effect on Markdown output (regression test)                     |
| Progress with Layout — MarkdownPlus        | Layout has no effect on MarkdownPlus output (regression test)                 |

---

### `bt` CLI

- This specification will ensure that the **Progress** component:
    - has a 'bt' CLI subcommand for rendering this component
    - that the '--md' and '--html' CLI switches are available to render to Markdown and HTML targets respectively (the default render is always for the Terminal)
    - that the '--example' CLI switch is in place to provide a thoughtful example of how this command should be used with the CLI (see other working examples for a template)

#### Current State

| Aspect              | Status                                                                |
|---------------------|-----------------------------------------------------------------------|
| CLI command exists  | Yes — `bt progress`                                                   |
| Render method       | Tree — calls `render_tree_node()` then `render_terminal_node()`      |
| Has `--md` switch   | No                                                                    |
| Has `--html` switch | No                                                                    |
| Has `--example`     | Yes (`bt progress --example`)                                         |

The existing `bt progress` command (`biscuit-terminal/cli/src/commands/progress.rs`)
creates a `Progress`, projects it to a `RenderNode`, and renders via
`render_terminal_node()`. It has `--example`, `--label`, `--width`,
`--fill-color`, `--empty-color`, and `--bracket-color` flags. It does not have
`--md`, `--md-plus`, or `--html` switches.

#### Specification Design

Add `--md`, `--md-plus`, and `--html` switches to the existing `bt progress`
command, following the pattern established by `bt prose`.

**Updated args:**

| Flag                          | Type           | Description                                                       |
|-------------------------------|----------------|-------------------------------------------------------------------|
| `PERCENT`                     | `Option<u8>`   | Completion percentage 0-100 (required unless `--example`)         |
| `--example` / `-e`            | `bool`         | Render example and show command                                   |
| `--label`                     | `Option<String>` | Label shown before the bar                                       |
| `--width`                     | `Option<u32>`  | Width of the bar portion in characters                            |
| `--fill-color`                | `Option<String>` | Color of filled portion (named or `#rrggbb`)                    |
| `--empty-color`               | `Option<String>` | Color of empty portion (named or `#rrggbb`)                     |
| `--bracket-color`             | `Option<String>` | Color of bracket glyphs (named or `#rrggbb`)                    |
| `--html`                      | `bool`         | Render to HTML fragment (conflicts with `--md`, `--md-plus`)      |
| `--md`                        | `bool`         | Render to portable Markdown (conflicts with `--html`, `--md-plus`)|
| `--md-plus`                   | `bool`         | Render to MarkdownPlus (conflicts with `--html`, `--md`)          |

**Render path:**

1. Build `Progress` from flags (percent, label, width, colors).
2. **Terminal** (default): Project tree → `render_terminal_node()` (existing path).
3. **HTML** (`--html`): Wrap in `BrowserTreeComponent` → `render_html_fragment()`.
4. **Markdown** (`--md`): Project tree → `render_markdown_node()`.
5. **MarkdownPlus** (`--md-plus`): Project tree → `render_markdown_node()` with
   MarkdownPlus mode.

**Implementation in `progress.rs`:**

```rust
#[derive(ClapArgs, Debug, Clone)]
pub struct ProgressArgs {
    #[arg(long, short = 'e')]
    pub example: bool,

    #[arg(value_name = "PERCENT", required_unless_present = "example")]
    pub percent: Option<u8>,

    #[arg(long)]
    pub label: Option<String>,

    #[arg(long)]
    pub width: Option<u32>,

    #[arg(long = "fill-color")]
    pub fill_color: Option<String>,

    #[arg(long = "empty-color")]
    pub empty_color: Option<String>,

    #[arg(long = "bracket-color")]
    pub bracket_color: Option<String>,

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with_all = ["md", "md_plus"])]
    pub html: bool,

    /// Render to portable Markdown instead of the terminal.
    #[arg(long, conflicts_with_all = ["html", "md_plus"])]
    pub md: bool,

    /// Render to MarkdownPlus instead of the terminal.
    #[arg(long = "md-plus", conflicts_with_all = ["html", "md"])]
    pub md_plus: bool,
}
```

The `run()` method adds cross-target branches after building the `Progress`:

```rust
impl Run for ProgressArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        // ... existing Progress construction ...

        let node = progress.render_tree_node().ok_or_else(|| {
            color_eyre::eyre::eyre!("Progress component produced no render-tree node")
        })?;
        let root = RenderNode::root(vec![node]);

        // Cross-target output
        if self.html {
            let component = BrowserTreeComponent::new(progress);
            println!("{}", component.render_html_fragment().render());
            return Ok(());
        }
        if self.md {
            let rendered = render_markdown_node(&root, &MarkdownRenderOptions::default())
                .map_err(|e| color_eyre::eyre::eyre!("markdown render failed: {e}"))?;
            println!("{}", rendered.output);
            return Ok(());
        }
        if self.md_plus {
            let rendered = render_markdown_node(&root, &MarkdownRenderOptions::default_plus())
                .map_err(|e| color_eyre::eyre::eyre!("markdown render failed: {e}"))?;
            println!("{}", rendered.output);
            return Ok(());
        }

        // Terminal (existing path)
        let term = detect_terminal_honoring_force_color();
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        let rendered = render_terminal_node(&root, &opts)
            .map_err(|e| color_eyre::eyre::eyre!("render failed: {e}"))?;
        println!("{}", rendered.output);

        if self.example {
            print_example_command(PROGRESS_EXAMPLE_CMD);
        }
        Ok(())
    }
}
```

**Example command** (unchanged):

```rust
const PROGRESS_EXAMPLE_CMD: &str =
    r#"bt progress 72 --label "Indexing" --width 28 --fill-color green --bracket-color cyan"#;
```

---

## Acceptance Criteria Summary

- [ ] `Progress`'s `TerminalRenderable::render()` delegates to the tree path by default
- [ ] Bespoke render path retained as `render_bespoke()` for parity testing
- [ ] `BrowserRenderable` achieved — browser tree renderer handles `ProgressHints` producing an HTML progress bar
- [ ] `MarkdownRenderable` implemented on `Progress` — Markdown outputs label + percentage, MarkdownPlus outputs inline HTML when colors are present
- [ ] `bt progress --html` renders HTML output (CSS-styled progress bar)
- [ ] `bt progress --md` renders Markdown output (`Loading 75%`)
- [ ] `bt progress --md-plus` renders MarkdownPlus output (inline HTML when colors present)
- [ ] `bt progress --example` continues to render example with command display
- [ ] Parity tests (bespoke vs tree) cover all variants listed in Terminal IR section
- [ ] `KNOWN_DRIFT` ledger documents accepted divergences
- [ ] `bt progress` existing flags (`--label`, `--width`, `--fill-color`, `--empty-color`, `--bracket-color`) continue to work unchanged

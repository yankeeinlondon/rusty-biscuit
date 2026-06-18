---
last_updated: "2026-05-17"
component: HorizontalRule
package: biscuit-terminal
status: research
---

# Challenges of Migrating the `HorizontalRule` Component to the Tree Rendering Architecture

## Functional and Design Goals

The `HorizontalRule` component provides a visually expressive, terminal-aware horizontal divider with configurable appearance. It was created to give CLI applications a way to visually separate content sections that goes beyond a plain `---` line, while degrading gracefully across diverse terminal environments.

### Design goals

1. **Visual expressiveness** — seven distinct visual styles (`Dashes`, `Dots`, `Waves`, `LineStar`, `LineCircle`, `InsetLine`, `CurtainRod`) let authors choose a rule that matches the tone of the surrounding content.
2. **Terminal-aware progressive enhancement** — a three-tier rendering strategy (image/Unicode/ASCII) adapts output to the detected terminal capabilities without the author needing to pick a tier.
3. **CSS-like sizing and alignment** — percentage, character, pixel, and bare-number width strings plus four alignment modes (`Full`, `Centered`, `Left`, `Right`) match authoring conventions from web CSS.
4. **Color support** — CSS basic-16 names and `#rrggbb` hex values wrap the rule body in ANSI escapes, with automatic downgrading (truecolor → nearest basic color) when the terminal does not support 24-bit color.
5. **Multi-target output** — the same component renders to both terminal (via `TerminalRenderable`) and browser (via `BrowserRenderable`), producing Kitty-protocol image escapes or Unicode glyphs for the terminal and an SVG with CSS custom properties for the browser.
6. **Weight (thickness) control** — `Thin` / `Medium` / `Thick` select heavier Unicode glyphs in the terminal and wider SVG strokes in the browser.

### Where it is used today

`HorizontalRule` is used in two distinct code paths across the monorepo:

1. **Direct component instantiation** (`biscuit-terminal`) — callers build an `HorizontalRule` with the builder pattern and call `.render(&term)` (terminal) or `.render_browser_svg()` (browser). The terminal render tree's `NodeKind::ThematicBreak` handler already delegates to `HorizontalRule::new().render(&terminal)` as a bare default.

2. **darkmatter's `RuleProcessor` → `InlineEvent::HorizontalRule` pipeline** — darkmatter parses Markdown horizontal rules that carry YAML attributes (e.g., `--- {style: waves, width: "75%"}`), emits `InlineEvent::HorizontalRule(HorizontalRuleAttrs)` events, and the terminal/HTML output serializers build an `HorizontalRule` from those attributes before rendering. This is the primary production path.

### Example usage

```rust
// Terminal rendering with full customization
let rule = HorizontalRule::new()
    .style(RuleStyle::Waves)
    .alignment(RuleAlignment::Centered)
    .weight(RuleWeight::Thick)
    .width("75%")
    .color("red");
let output = rule.render(&terminal);
```

```rust
// Browser rendering — produces an SVG with CSS custom properties
let svg = rule.render_browser_svg();
// <svg width="75%" ... style="--hr-weight: 8; --hr-color: red; --hr-width: 75%;">
```

```rust
// darkmatter Markdown → HorizontalRule via RuleProcessor
// Input:  --- {style: dots, weight: thin, color: "#00ff00"}
// Produces: InlineEvent::HorizontalRule(attrs) → build_rule(&attrs) → render
```

## Technical Implementation (current)

### Module structure

The component lives in `biscuit-terminal/lib/src/components/horizontal_rule/` and is split into three files:

| File | Responsibility |
|---|---|
| `mod.rs` | `HorizontalRule` struct, `TerminalRenderable` impl, width resolution, tier dispatch, Unicode/ASCII content generation, image-tier SVG construction, color wrapping |
| `style.rs` | `RuleStyle`, `RuleAlignment`, `RuleWeight` enums (all `#[derive(Debug, Clone, PartialEq)]`, serde-enabled with kebab/lowercase rename) |
| `browser.rs` | `BrowserRenderable` impl, SVG rendering with CSS `var()` custom properties, `parse_basic_color`, `parse_hex_color`, `nearest_basic_color`, `MarginToCss` trait |

### Struct definition

```rust
pub struct HorizontalRule {
    style: RuleStyle,          // visual pattern
    alignment: RuleAlignment,  // full / centered / left / right
    weight: RuleWeight,        // thin / medium / thick
    width: Option<String>,     // CSS-like width spec ("50%", "20ch", "200px", "40")
    color: Option<String>,     // CSS color ("red", "#ff0000")
    layout: Layout,            // margins, alignment, wrap policy
}
```

### Key responsibilities in `TerminalRenderable::render`

The `render(&self, term: &Terminal) -> String` method performs these transforms in order:

1. **Tier 1 dispatch** — if the terminal is a TTY with Kitty/iTerm2 image support, rasterize an SVG to PNG and emit it through `TerminalImage::render_kitty_cells`. This produces Kitty graphics protocol escape sequences with cursor save/restore and vertical motion. On any failure (rasterization error, missing capability), falls through.

2. **Width resolution** — `resolve_width(term_width)` parses `self.width`:
   - `"NN%"` → percentage of terminal width
   - `"NNch"` → explicit character count
   - `"NNpx"` → pixel count converted to columns using `DEFAULT_CELL_WIDTH` (8px/cell)
   - Bare `"NN"` → character count
   - Unrecognized strings log a `tracing::warn!` and fall back to full width
   - `None` → full width for `Full` alignment, 80% for other alignments

3. **Content generation** — `generate_terminal_content(width, term)` selects glyphs:
   - **Tier 2 (Unicode)**: when `term.supports_unicode` is true, uses box-drawing characters (`╌`/`╍`, `·`/`•`, `≋`, `─`/`━`, `★`, `●`, `┤`/`├`). `RuleWeight::Thick` swaps in heavy variants.
   - **Tier 3 (ASCII)**: when Unicode is unavailable, uses plain ASCII (`-`, `.`, `~`, `*`, `o`, `[`/`]`). ASCII is weight-insensitive by design.

4. **Color wrapping** — `apply_terminal_color(content, term)` wraps the rule body in ANSI foreground escapes. Named colors map to `BasicColor`; hex colors map to `RgbColor` (truecolor) or nearest `BasicColor` (basic color terminals). The padding stays outside the color wrap.

5. **Alignment** — prepends or surrounds the content with spaces for `Centered`/`Right` alignment using `visible_width()` (which strips ANSI escapes from the width count).

### Key responsibilities in `BrowserRenderable`

The browser path (`render_browser_svg`) builds a self-contained SVG with:

- Concrete `width`/`height` attributes
- CSS custom property declarations (`--hr-weight`, `--hr-color`, `--hr-width`) on the root `<svg>` style
- `var(--hr-xxx, fallback)` references in stroke/fill so the SVG degrades when inline styles are stripped
- Per-style SVG shape primitives (lines, paths, circles)

### How the render tree currently handles `ThematicBreak`

The terminal render tree (`render.rs:305`) currently does:

```rust
NodeKind::ThematicBreak => {
    let rule = HorizontalRule::new();
    Ok(rule.render(&self.opts.context.terminal))
}
```

This constructs a **bare default** `HorizontalRule` and renders it directly. All style, width, color, alignment, and weight attributes from the original Markdown source are lost at this point because `NodeKind::ThematicBreak` is a leaf variant with no fields and no `NodeAttrs` carrying HR-specific data.

The browser render tree (`browser.rs:226`) emits `<hr>` via `self.void(VoidTag::Hr, &node.attrs)` — similarly bare.

The Markdown render tree (`markdown.rs:229`) emits `---` — a plain Markdown thematic break.

## Implementation Challenges

### ThematicBreak Is an Attribute-less Leaf Node

**Description**: `NodeKind::ThematicBreak` is defined as a unit variant with no fields and no children. It carries no data about style, weight, alignment, width, or color. The render tree model treats it as semantically equivalent to a Markdown `---` — a plain, unstyled divider.

**Example**: A darkmatter document with `--- {style: waves, width: "75%", color: "red"}` parses through `RuleProcessor`, which extracts the YAML attributes into `HorizontalRuleAttrs`. The terminal serializer (`output/terminal.rs`) uses `build_rule(&attrs)` to construct a fully configured `HorizontalRule`. But if the same document is folded into the render tree, the fold produces `NodeKind::ThematicBreak` — all attributes are discarded. The terminal tree renderer then renders `HorizontalRule::new()` (default Dashes, Full width, no color).

**Suggested test**:

```rust
#[test]
fn thematic_break_node_carries_no_style_attributes() {
    let node = RenderNode::thematic_break();
    assert!(matches!(node.kind, NodeKind::ThematicBreak));
    // There is no way to recover style, width, color, weight from this node.
    let rendered = render_terminal_node(&node, &TerminalRenderOptions::default())
        .unwrap()
        .output;
    let term = Terminal::new_optimistic(80);
    let expected = HorizontalRule::new().render(&term);
    assert_eq!(rendered, expected, "ThematicBreak renders as a bare default HR");
}
```

---

### Multi-Tier Rendering Requires Runtime Terminal Capability Queries

**Description**: The current `render()` method makes runtime decisions based on `Terminal` properties (`is_tty`, `image_support`, `color_depth`, `supports_unicode`). The tree rendering model separates "build the tree" (target-agnostic) from "walk the tree" (target-specific). For `HorizontalRule`, the choice between Tier 1 (Kitty image), Tier 2 (Unicode), and Tier 3 (ASCII) happens inside `render()`, which is already target-specific — but the tree renderer would need to re-implement this tier dispatch or delegate back to the component, which defeats the purpose of the tree abstraction.

**Example**: A rule rendered through the tree on a Kitty-capable terminal should emit `\x1b_G` image escapes. The same tree node rendered on a non-TTY should emit ASCII hyphens. The tree `Writer` does not have access to the Tier 1 rasterization pipeline (SVG → PNG → Kitty cells) because that pipeline is embedded in `HorizontalRule`'s private methods.

**Suggested test**:

```rust
#[test]
fn tree_rendered_hr_uses_correct_tier_for_kitty_terminal() {
    let node = RenderNode::thematic_break();
    let term = Terminal::builder()
        .width(80)
        .is_tty(true)
        .image_support(ImageSupport::Kitty)
        .build();
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    let result = render_terminal_node(&node, &opts).unwrap().output;
    assert!(
        result.contains("\x1b_G"),
        "Kitty terminal should produce image-tier output: {result:?}"
    );
}
```

---

### Image Tier Has Complex Cursor and Cell-Size Dependencies

**Description**: Tier 1 rendering generates Kitty graphics protocol escape sequences that include cursor save/restore (`\x1b[s` / `\x1b[u`), horizontal cursor movement (`\x1b[N C`) for alignment, and vertical motion (`\x1b[N B`) after the image. This sequence depends on the resolved width in cells, the height in cells (derived from `RuleWeight`), and the terminal's actual cell size in pixels. These escape sequences must not appear in non-terminal output formats (Markdown, HTML).

**Example**: On a Kitty terminal with `rule_width=40`, `weight=Medium` (2 cells high), the Tier 1 output is:

```
\x1b[s\x1b[20C\x1b_G...image bytes...\x1b[u\x1b[2B\r
```

The `\x1b[20C` prefix centers a 40-cell rule on an 80-column terminal. The `\x1b[2B\r` moves the cursor down 2 rows. If the tree renderer simply calls `HorizontalRule::render()`, these terminal-specific escapes leak into the tree output — but they must not appear when the same tree node is rendered to Markdown (`---`) or HTML (`<hr>`).

**Suggested test**:

```rust
#[test]
fn tree_rendered_hr_does_not_leak_kitty_escapes_to_markdown() {
    let node = RenderNode::thematic_break();
    let rendered = render_markdown_node(&node, &MarkdownRenderOptions::default())
        .unwrap()
        .output;
    assert!(
        !rendered.contains("\x1b_G"),
        "Markdown output must not contain Kitty image escapes: {rendered:?}"
    );
    assert!(
        !rendered.contains("\x1b[s"),
        "Markdown output must not contain cursor save sequences: {rendered:?}"
    );
}
```

---

### CSS Custom Property Strategy Has No Tree Equivalent

**Description**: The browser rendering path emits SVG with CSS custom properties (`--hr-weight`, `--hr-color`, `--hr-width`) and `var()` fallback references. This is a design that allows external CSS to override the rule's appearance. The tree's browser renderer (`render_browser_node`) currently produces a bare `<hr>` void element for `ThematicBreak`. There is no mechanism in the tree model to carry or project these CSS custom property semantics.

**Example**: The bespoke `render_browser_svg()` produces:

```html
<svg width="75%" height="40" style="--hr-weight: 4; --hr-color: red; --hr-width: 75%;">
  <line stroke="var(--hr-color, red)" stroke-width="var(--hr-weight, 4)" .../>
</svg>
```

The tree browser renderer produces:

```html
<hr>
```

All styling, custom properties, and SVG shape information is lost.

**Suggested test**:

```rust
#[test]
fn tree_browser_rendered_thematic_break_is_bare_hr() {
    let node = RenderNode::thematic_break();
    let rendered = render_browser_node(&node, &BrowserRenderOptions::default())
        .unwrap()
        .output;
    assert!(
        rendered.contains("<hr"),
        "Browser ThematicBreak should contain <hr>: {rendered:?}"
    );
    assert!(
        !rendered.contains("--hr-weight"),
        "Bare <hr> does not carry CSS custom properties: {rendered:?}"
    );
}
```

---

### Width Resolution Depends on Terminal Width at Render Time

**Description**: `resolve_width()` converts CSS-like width specifications (`"50%"`, `"20ch"`, `"200px"`) to a concrete column count using the terminal's current width. The tree model separates content from presentation — the same tree node might be rendered at different widths. A percentage-based width must be re-resolved each time the tree is walked, not baked into the node at projection time.

**Example**: A rule configured with `.width("50%")` produces 50 columns on a 100-column terminal and 40 columns on an 80-column terminal. If the tree projection bakes in `50` (from a 100-column context), the rule will be 10 columns too wide when rendered on an 80-column terminal.

**Suggested test**:

```rust
#[test]
fn width_resolution_changes_with_terminal_width() {
    // Simulate what would happen if a tree were rendered at two widths.
    let rule = HorizontalRule::new().width("50%");
    let wide_term = Terminal::new_optimistic(100);
    let narrow_term = Terminal::new_optimistic(80);
    let wide_output = rule.render(&wide_term);
    let narrow_output = rule.render(&narrow_term);
    assert_ne!(
        wide_output, narrow_output,
        "50% width must produce different output at different terminal widths"
    );
    assert_eq!(
        rule.visible_width(&strip_ansi_codes(&wide_output)),
        50,
        "50% of 100 columns = 50"
    );
    assert_eq!(
        rule.visible_width(&strip_ansi_codes(&narrow_output)),
        40,
        "50% of 80 columns = 40"
    );
}
```

---

### Color and Weight Are Render-Time Decorations Without Tree Representation

**Description**: The color (ANSI wraps for terminal, stroke attribute for browser SVG) and weight (heavy Unicode glyphs, SVG stroke width) are applied at render time by the bespoke component. `NodeKind::ThematicBreak` has no fields to carry these attributes. `NodeAttrs` supports `classes` and `data` (namespaced extension data), but no HR-specific typed properties exist. Without extending `NodeAttrs` or `ThematicBreak`, these properties cannot survive the projection into the tree.

**Example**: A rule with `.color("red").weight(RuleWeight::Thick)` produces:
- Terminal: `\x1b[31m╍╍╍╍╍╍╍╍\x1b[39m` (red heavy dashes)
- Browser: `<svg ... style="--hr-weight: 8; --hr-color: red;">...`

After projection to `ThematicBreak`, both renderers lose this information and produce unstyled default output.

**Suggested test**:

```rust
#[test]
fn colored_rule_loses_color_through_tree_projection() {
    let rule = HorizontalRule::new()
        .style(RuleStyle::Dashes)
        .color("red")
        .weight(RuleWeight::Thick);
    let term = Terminal::new_optimistic(80);
    let bespoke_output = rule.render(&term);

    // Project to tree and render back.
    let node = RenderNode::thematic_break();
    let tree_output = render_terminal_node(
        &node,
        &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
    )
    .unwrap()
    .output;

    // The bespoke output has color escapes; the tree output does not.
    assert!(
        bespoke_output.contains("\x1b[31m"),
        "bespoke output has red color"
    );
    assert!(
        !tree_output.contains("\x1b[31m"),
        "tree projection loses the color attribute"
    );
}
```

---

### Structural Styles (InsetLine, CurtainRod) Embed Layout Logic in Content

**Description**: Some `RuleStyle` variants embed layout structure directly into the generated content. `InsetLine` prepends/trails 2 spaces of padding (`"  ────  "`), and `CurtainRod` wraps the body in bracket caps (`┤────├` / `[----]`). This means alignment and width logic interacts with the content itself — the "visible width" of the rule body is not simply `width × glyph_width`. The tree model would need to either (a) carry these structural variations as node-level metadata that each renderer interprets, or (b) accept that the tree representation is lossy for these styles.

**Example**: An 80-column `InsetLine` rule produces `"  " + "─".repeat(76) + "  "` — 4 of the 80 columns are consumed by the inset padding, not the line glyphs. A `CurtainRod` on the same width produces `"┤" + "─".repeat(76) + "├"` — 2 cells for caps + 76 for the body. If the tree renderer naively renders `ThematicBreak` as a full-width line of dashes, neither style is preserved.

**Suggested test**:

```rust
#[test]
fn inset_line_preserves_inset_padding_through_tree() {
    let rule = HorizontalRule::new()
        .style(RuleStyle::InsetLine)
        .alignment(RuleAlignment::Full);
    let term = Terminal::new_optimistic(80);
    let bespoke = rule.render(&term);
    assert!(
        bespoke.starts_with("  "),
        "InsetLine has 2-char left inset: {bespoke:?}"
    );
    assert!(
        bespoke.ends_with("  "),
        "InsetLine has 2-char right inset: {bespoke:?}"
    );

    // Through the tree, this structure is lost.
    let node = RenderNode::thematic_break();
    let tree_output = render_terminal_node(
        &node,
        &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
    )
    .unwrap()
    .output;
    assert!(
        !tree_output.starts_with("  "),
        "tree ThematicBreak has no inset padding: {tree_output:?}"
    );
}
```

---

### Parity with darkmatter's RuleProcessor and HorizontalRuleAttrs

**Description**: darkmatter's `RuleProcessor` parses YAML attributes from Markdown horizontal rules and produces `HorizontalRuleAttrs` (style, weight, color, alignment, width). The `build_rule()` function translates these into an `HorizontalRule`. The render tree's `fold_markdown_to_document` currently folds `---` into a bare `NodeKind::ThematicBreak`, discarding all attributes. Reconciling this with the tree would require either extending the fold to carry HR attributes or keeping the `RuleProcessor` pipeline separate indefinitely.

**Example**: Given this Markdown:

```markdown
--- {style: waves, width: "75%", color: "red", weight: thick}
```

darkmatter's `RuleProcessor` produces `InlineEvent::HorizontalRule(attrs)` where `attrs.style == Some("waves")`, etc. The `build_rule(&attrs)` constructs a fully configured `HorizontalRule`. But the tree fold produces `ThematicBreak` — the attributes are silently dropped because the fold has no way to attach them.

**Suggested test**:

```rust
#[test]
fn darkmatter_rule_attrs_survive_through_bespoke_pipeline() {
    let attrs = HorizontalRuleAttrs {
        style: Some("waves".to_string()),
        width: Some("75%".to_string()),
        color: Some("red".to_string()),
        weight: Some("thick".to_string()),
        alignment: Some("centered".to_string()),
    };
    let rule = build_rule(&attrs);
    assert_eq!(rule.rule_style(), &RuleStyle::Waves);
    assert_eq!(rule.rule_width(), Some("75%"));
    assert_eq!(rule.rule_color(), Some("red"));
    assert_eq!(rule.rule_weight(), &RuleWeight::Thick);
}
```

---

### Browser Rendering Is SVG-Island, Not HTML-Element

**Description**: The bespoke `BrowserRenderable` implementation for `HorizontalRule` returns a `BrowserFragment` wrapping `ComposableNode::RawHtml` — a complete, self-contained SVG island. The tree's browser renderer emits a semantic `<hr>` HTML element. These are fundamentally different rendering strategies: one is a rich SVG graphic with CSS custom properties; the other is a semantic HTML void element. Migrating to the tree would either (a) force the browser renderer to construct the SVG from `ThematicBreak` (re-implementing all the SVG generation logic), or (b) accept that the tree's browser output is a semantic `<hr>` while the bespoke path produces a richer SVG.

**Example**: Compare the two outputs for the same logical content:

```
// Bespoke (SVG island):
<svg width="100%" height="40" style="--hr-weight: 4; --hr-color: currentColor; --hr-width: 100%;">
  <line x1="0" y1="50%" x2="100%" y2="50%" stroke="var(--hr-color, currentColor)" .../>
</svg>

// Tree (semantic HTML):
<hr>
```

The tree output is correct HTML but visually inert compared to the bespoke SVG.

**Suggested test**:

```rust
#[test]
fn bespoke_browser_output_is_svg_island() {
    let rule = HorizontalRule::new().style(RuleStyle::Waves).color("blue");
    let fragment = rule.render_html_fragment();
    let html = fragment.to_string();
    assert!(
        html.contains("<svg"),
        "bespoke browser output is an SVG island: {html:?}"
    );
    assert!(
        html.contains("var(--hr-color, blue)"),
        "SVG uses CSS custom properties: {html:?}"
    );
}
```

## Solution Suggestions

#### Extend ThematicBreak with Optional Attributes

**Description**: Promote `NodeKind::ThematicBreak` from a bare unit variant to a struct variant carrying optional HR attributes:

```rust
ThematicBreak {
    style: Option<String>,
    weight: Option<String>,
    alignment: Option<String>,
    width: Option<String>,
    color: Option<String>,
}
```

Each renderer would interpret these strings to configure the output. The terminal renderer would construct an `HorizontalRule` via `build_rule()` before calling `.render()`. The browser renderer could emit either a styled `<hr>` with CSS inline styles or the full SVG island. The Markdown renderer would continue emitting `---`.

**Which challenges this helps with**:

- **ThematicBreak Is an Attribute-less Leaf Node** — directly addresses by adding fields
- **Color and Weight Are Render-Time Decorations** — attributes survive projection
- **Parity with darkmatter's RuleProcessor** — the fold can populate these from `HorizontalRuleAttrs`

**Variant solutions**: Instead of typed fields on `NodeKind`, use `NodeAttrs.data` with a namespaced key (e.g., `"hr"`) carrying a `serde_json::Value` of the attributes. This avoids extending the `NodeKind` enum but trades type safety for unstructured data.

---

#### Terminal Renderer Delegates to HorizontalRule Component

**Description**: Rather than re-implementing tier dispatch, glyph selection, and image-tier rasterization inside the tree's `Writer`, the terminal renderer's `ThematicBreak` handler would construct an `HorizontalRule` from the node's attributes (or defaults) and call `.render(&terminal)`. This is what the current code already does for the bare case — it just needs the attribute bridge.

**Which challenges this helps with**:

- **Multi-Tier Rendering Requires Runtime Terminal Capability Queries** — no re-implementation needed; the existing component handles all three tiers
- **Image Tier Has Complex Cursor and Cell-Size Dependencies** — the image-tier pipeline stays inside `HorizontalRule`
- **Width Resolution Depends on Terminal Width at Render Time** — `resolve_width()` runs at render time, not projection time

**Variant solutions**: Introduce a `CodeRenderer`-style hook (`HrRenderer`) that the terminal options carry, allowing custom rule rendering without baking `HorizontalRule` into the tree renderer.

---

#### Browser Renderer Emits Styled `<hr>` with CSS Custom Properties

**Description**: Instead of emitting a bare `<hr>` void element, the tree's browser renderer could emit a styled `<hr>` with inline CSS custom properties derived from the node's HR attributes:

```html
<hr style="--hr-weight: 4; --hr-color: red; border: none; border-top: var(--hr-weight, 4px) solid var(--hr-color, currentColor);">
```

This preserves the CSS custom property strategy without requiring the full SVG island. For richer styles (`Waves`, `LineStar`, `CurtainRod`), the renderer could fall back to the SVG island approach.

**Which challenges this helps with**:

- **CSS Custom Property Strategy Has No Tree Equivalent** — the browser renderer can emit CSS custom properties on the `<hr>` element itself
- **Browser Rendering Is SVG-Island, Not HTML-Element** — provides a middle ground between bare `<hr>` and full SVG

**Variant solutions**: Introduce a per-component "browser hint" in `NodeAttrs` that tells the browser renderer whether to emit an SVG island or a styled `<hr>`. The terminal renderer already uses `columns_hints()` and `table_column_hints()` for a similar purpose.

---

#### NodeAttrs Extension Data for HR-Specific Properties

**Description**: Add a well-known extension key (e.g., `"hr"`) to `NodeAttrs.data` that carries HR attributes as structured data. The tree's `NodeAttrs` already supports namespaced extension data via a `HashMap<String, serde_json::Value>`. Each renderer would check for this key and use the data to configure the output. This avoids extending `NodeKind::ThematicBreak` while still allowing attributes to survive projection.

**Which challenges this helps with**:

- **ThematicBreak Is an Attribute-less Leaf Node** — attributes travel through `NodeAttrs` instead of `NodeKind`
- **Parity with darkmatter's RuleProcessor** — the fold can populate `NodeAttrs.data["hr"]` from `HorizontalRuleAttrs`
- **Structural Styles Embed Layout Logic in Content** — the style name travels through data, letting each renderer interpret it

**Variant solutions**: Define a typed `ThematicBreakAttrs` struct with a `From<HorizontalRuleAttrs>` conversion, stored in `NodeAttrs.data` under a reserved key. This gives type safety at the renderer boundary while keeping `NodeKind` stable.

---

#### Component Parity Test for HorizontalRule

**Description**: Follow the established pattern from `BlockQuote`'s parity test (`render_tree_component_parity.rs`). Build an `HorizontalRule` with full attributes, render it both ways (bespoke `TerminalRenderable` vs. `TreeComponent` through the tree), and compare outputs on semantic invariants (visible content, ANSI-stripped text). This does not solve a challenge directly but establishes the parity discipline for this component.

**Which challenges this helps with**:

- All challenges — the parity test is the quality gate that proves any migration is faithful. Without it, challenges like attribute loss, tier mismatch, and style degradation would go undetected.

**Variant solutions**: Use a snapshot-based parity test (compare golden output files) for richer assertion of visual structure, not just semantic text equivalence.

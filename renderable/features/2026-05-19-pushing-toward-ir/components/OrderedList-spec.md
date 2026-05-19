# OrderedList — IR Rendering Design Specification

## Component Status

| Field      | Value                                                                            |
|------------|----------------------------------------------------------------------------------|
| Name       | OrderedList                                                                      |
| Kind       | Block                                                                            |
| Location   | `biscuit-terminal/lib/src/components/list.rs`                                    |
| Terminal   | ✅ bespoke `TerminalRenderable`                                                   |
| Browser    | ❌                                                                                |
| Markdown   | ❌                                                                                |
| Tree       | ✅ `render_tree_node()` exists (projects to `NodeKind::List { ordered: true }`)   |
| IR State   | both avail, old renders                                                          |
| bt CLI     | bespoke (via `bt list` which only renders `UnorderedList`)                       |

OrderedList holds a `Vec<RenderableTerminalContent>` and renders items with
numeric prefixes (`1.`, `2.`, `3.`, etc.). It owns a `Layout` for margins,
alignment, and word-wrap, and an `indent_children: u32` (default 4) that
controls the indentation of nested block-level children.

A tree projection already exists via `render_tree_node()`, producing a
`NodeKind::List { ordered: true, start: None, children }` with each item
projected into `NodeKind::ListItem` nodes. The projection also seeds the
component's `Layout` onto the root node and records `ListRenderHints` with
`hanging_indent: true` and `indent_children: Some(4)`. However, the default
`TerminalRenderable::render()` still uses the bespoke `render_content()` path.

The terminal tree renderer (`render_tree::render`) already has **native list
rendering** — it handles `NodeKind::List` and `NodeKind::ListItem` directly,
producing numbered/bulleted output with hanging indent and block-child
indentation without delegating back to the bespoke `OrderedList` component.

---

## Design Steps

### Terminal IR Implementation

- The **OrderedList** component does not currently have a IR based rendering solution
- This section will describe what is required to ensure that the **OrderedList** component:
    - has an IR implementation
    - the IR implementation drives the TerminalRenderable contract
    - the IR implementation is what is used by the bt CLI (note if **OrderedList** doesn't yet have bt CLI subcommand then it will be designed below in the bt CLI section)

#### Tree Projection Status

OrderedList already has a working tree projection via `render_tree_node()` at
`list.rs:274`. The projection:

1. Iterates `self.items`, calling `project_list_items()` on each.
2. Each item is projected via `RenderableTerminalContent::to_tree_nodes()` into
   a `NodeKind::ListItem`.
3. The list node is `RenderNode::list(true, None, children)` — ordered, no
   explicit start.
4. `ListRenderHints` are set: `bullet: None`, `hanging_indent: true`,
   `indent_children: Some(self.indent_children)`.
5. Layout is seeded if non-default.

The native terminal tree renderer already handles this structure at
`render_tree::render.rs:762` (`render_list`) and `:809` (`render_list_item`).

#### Switching the Default Render Path

The goal is to make the IR path the default for both `TerminalRenderable::render()`
and the bt CLI, while **retaining the bespoke path** for parity testing.

The switch follows the `TreeComponent` adapter pattern: OrderedList's
`TerminalRenderable::render()` and `render_optimistic()` delegates are changed
to project the tree and call `render_terminal_node()`.

**Implementation approach:**

```rust
impl TerminalRenderable for OrderedList {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = Terminal::new_optimistic(width);
        self.render_via_tree(&term)
    }

    fn render(&self, term: &Terminal) -> String {
        self.render_via_tree(term)
    }

    // ... layout, is_block_level, as_any unchanged ...

    fn render_tree_node(&self) -> Option<RenderNode> {
        // existing projection unchanged
    }
}

impl OrderedList {
    fn render_via_tree(&self, term: &Terminal) -> String {
        let node = self.render_tree_node().expect("OrderedList always projects");
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => format!("[render-tree error: {error}]"),
        }
    }

    /// Retained for parity testing. Renders via the pre-tree bespoke path.
    fn render_bespoke(&self, term: &Terminal) -> String {
        let width = term.width();
        let available = self.layout.available_width(width);
        let content = self.render_content(Some(term), available);
        self.layout.apply_block_layout(&content, width)
    }
}
```

#### Layout Mapping

OrderedList's `Layout` already seeds onto the projected `RenderNode` via
`node.attrs.set_layout(&self.layout)`. The terminal tree renderer's
`render_with_layout` applies it during rendering. No additional Layout
parameters are needed.

The `indent_children` parameter is carried via `ListRenderHints` on the node
attributes, which the native terminal renderer reads at `render.rs:776`:
`hints.indent_children.unwrap_or(default_indent)`.

#### Style Considerations

OrderedList has no visual Style of its own (no color, border, fill, or
emphasis). It is a structural container. No `Style` is seeded on the projected
node.

#### Parity Test Strategy

Critical test variants for the IR vs bespoke comparison:

| Variant                                                      | Validates                                                                               |
|--------------------------------------------------------------|-----------------------------------------------------------------------------------------|
| Empty OrderedList                                            | Both paths produce `""`                                                                 |
| Single string item                                           | Output is `"1. Item\n"` in both paths                                                   |
| Three string items                                           | Numbering `1.`, `2.`, `3.` preserved in both                                            |
| Long string item requiring word wrap                         | Hanging indent on continuation lines matches                                            |
| Item count ≥ 10 (double-digit prefix)                        | Prefix width grows (`10. ` = 4 chars); alignment remains correct                        |
| String item + Prose component item                           | Content survives both paths; Prose styling loss in tree path is documented              |
| String item + nested OrderedList (block child)               | Block child is indented by `indent_children`; numbering restarts at 1                   |
| String item + nested UnorderedList (block child)             | Block child renders with bullet prefix at correct indent                                |
| Three-level nesting (OL > OL > OL)                           | Indent compounds: 4 + 4 + 4 = 12 spaces; no width overflow                             |
| Empty nested OrderedList                                     | Produces blank line between siblings; numbering continues                               |
| OrderedList with left/right margins                          | Layout is applied via `set_layout`; margins narrow the available width                  |
| OrderedList with alignment                                   | Block alignment applied as a unit                                                       |
| OrderedList with custom `indent_children`                    | Block children indented by specified amount                                             |
| Mixed inline + block children                                | First child gets prefix, block children get indent, no prefix                           |
| Item count ≥ 100 (triple-digit prefix)                       | Prefix `"100. "` = 5 chars; wrapping and alignment correct                              |

Parity is asserted on **ANSI-stripped content equality** (not byte-identical
output), following the BlockQuote parity discipline. Known accepted divergences
should be documented in a `KNOWN_DRIFT` ledger:

- **Prose styling loss**: Items that are Prose components lose styling in the
  tree path because Prose's `render_tree_node()` returns `None`, triggering
  ANSI-stripped fallback. Content is preserved; styling is not.
- **Hanging indent computation**: The bespoke path computes hanging indent per
  item (`format!("{number}. ")`), while the tree renderer computes it from
  `origin + offset`. Both should produce the same result, but the code paths
  differ.
- **Width handling**: The bespoke path manually computes `child_width =
  term_width - prefix_width` for each item. The tree renderer's width
  management may produce slightly different line breaks in edge cases at very
  narrow widths.

#### Feature Requests for Tree Rendering

No feature requests are needed. The existing tree renderer already has native
support for ordered lists with:

- Numeric prefix generation from `start` / `offset`
- Hanging indent for wrapped continuation lines
- Block-child indentation via `ListRenderHints::indent_children`
- Mixed inline/block child handling in `render_list_item`
- Layout application via `set_layout`

The `NodeKind::List { ordered: true }` node kind and the native rendering in
`render_tree::render.rs` are a direct match for OrderedList's semantics.

#### Tree Renderer Fit Assessment

The existing tree renderer is an **excellent fit** for OrderedList. The
component's entire rendering logic — numeric prefixes, hanging indent,
block-child indentation, and Layout application — has a corresponding native
implementation in the tree renderer. The `NodeKind::List` / `NodeKind::ListItem`
node types were designed to represent exactly this kind of component.

OrderedList's tree projection already exists and is tested (see
`ordered_list_render_tree_node_carries_layout_when_margins_set` at `list.rs:885`).
The remaining work is switching the default render path and adding comprehensive
parity tests.

`will_use_tree_renderer`: **true** — the existing tree renderer handles
OrderedList's needs without any feature additions.

`will_use_tree_renderer_with_features`: **true** — no features requested, so
this is the same as above.

---

### Browser IR Implementation

- In this section we will provide a design specification for the **OrderedList** component's implementation of the BrowserRenderable trait

OrderedList does not currently have a bespoke browser rendering implementation.
Since Terminal IR is designed first and OrderedList already projects to a
`NodeKind::List { ordered: true }` render tree node, the browser path is handled
entirely by the existing `BrowserTreeComponent<T>` adapter in
`biscuit-terminal/lib/src/render_tree/browser_adapter.rs`.

The browser tree renderer already handles `NodeKind::List` at
`renderable/src/tree/render/browser.rs:363`:

- Ordered lists (`ordered: true`) produce `<ol>` with an optional `start`
  attribute when the start index is not 1.
- Each `NodeKind::ListItem` produces `<li>` with optional checkbox for task
  items.
- Child content (text, paragraphs, nested lists) is rendered recursively.

OrderedList gains `BrowserRenderable` by wrapping itself in the adapter:

```rust
use biscuit_terminal::render_tree::BrowserTreeComponent;
use renderable::browser::BrowserRenderable;

let ol = OrderedList::new(vec!["First", "Second", "Third"]);
let component = BrowserTreeComponent::new(ol);
let fragment = component.render_html_fragment();
let html = fragment.render();
// Produces: <ol><li>First</li><li>Second</li><li>Third</li></ol>
```

#### Layout to CSS Mapping

OrderedList's `Layout` maps to CSS via the existing `layout_to_css` lowering in
`renderable/src/tree/render/browser.rs`:

- Margins → `margin-*` properties on the `<ol>` wrapper
- Alignment → `text-align` when `max_width` is present
- `max_width` → `max-width` CSS property

No additional CSS mapping is needed beyond what the tree renderer already provides.

#### Key Test Variants

| Variant                                      | Asserts                                                                          |
|----------------------------------------------|----------------------------------------------------------------------------------|
| Empty OrderedList                            | Produces `<ol></ol>` with no children                                            |
| Single string item                           | HTML contains `<ol><li>Item</li></ol>`                                           |
| Three string items                           | Three `<li>` elements inside `<ol>` with correct text                           |
| Nested OrderedList (block child)             | HTML contains nested `<ol>` inside an `<li>`                                    |
| Nested UnorderedList (block child)           | HTML contains `<ul>` inside an `<li>`                                           |
| String + Prose item                          | HTML contains text content (Prose renders as fallback text since no tree)        |
| Layout with margins                          | `<ol>` wrapper has `margin-left` / `margin-right` CSS                           |
| Layout with alignment and max-width          | `<ol>` wrapper has `text-align` and `max-width` CSS                             |
| Custom `indent_children`                     | No effect on HTML output (indent is a terminal concern)                          |
| Long text item                               | Content is present in the `<li>`; wrapping is CSS-driven                        |

---

### Markdown IR Implementation

#### Markdown vs MarkdownPlus for OrderedList

OrderedList is a structural container with no color, border, fill, or visual
styling of its own. This means:

- **Both Markdown and MarkdownPlus produce identical output** for OrderedList.
- An ordered list is natively representable in CommonMark: `1. Item`.
- There is no situation where OrderedList's own structure would diverge between
  the two targets.

The only potential divergence would come from child components that have color
or style (e.g., a Prose with `<red>error</red>`). Since the tree-based Markdown
renderer ignores `Style` entirely (locked by regression test), and OrderedList
adds no style of its own, the two outputs are identical through the tree path.

#### Markdown Rendering Design

The Markdown tree renderer already handles `NodeKind::List` at
`renderable/src/tree/render/markdown.rs:308`:

- Ordered lists produce `1. First\n2. Second\n3. Third`
- Continuation lines are indented to align under the marker
- The renderer respects the `start` attribute for numbering origin
- Block children within list items are indented correctly

OrderedList projects to `NodeKind::List { ordered: true, start: None }`, so the
Markdown renderer will produce standard numbered list syntax starting from 1.

```rust
use renderable::tree::render::{render_markdown_node, MarkdownRenderOptions};

let ol = OrderedList::new(vec!["First", "Second", "Third"]);
let node = ol.render_tree_node().unwrap();
let rendered = render_markdown_node(&node, &MarkdownRenderOptions::default());
// Produces: "1. First\n2. Second\n3. Third"
```

Layout is ignored by the Markdown renderer (by design — locked by test).

OrderedList can implement `MarkdownRenderable` by projecting its tree and
calling `render_markdown_node`:

```rust
impl MarkdownRenderable for OrderedList {
    fn render_markdown(&self) -> String {
        let node = self.render_tree_node().unwrap();
        render_markdown_node(&node, &MarkdownRenderOptions::default())
            .map(|r| r.output)
            .unwrap_or_default()
    }

    fn render_markdown_plus(&self) -> String {
        // Identical for OrderedList — no styling divergence
        self.render_markdown()
    }
}
```

#### Key Test Variants

| Variant                                  | Asserts                                                                           |
|------------------------------------------|-----------------------------------------------------------------------------------|
| Empty OrderedList                        | Produces `""`                                                                     |
| Single item                              | Markdown is `"1. Item"`                                                           |
| Three items                              | Markdown is `"1. First\n2. Second\n3. Third"`                                    |
| Long item requiring wrap                 | Continuation lines indented to marker width                                       |
| Nested OrderedList                       | Markdown contains indented numbered sublist                                       |
| Nested UnorderedList                     | Markdown contains indented bullet sublist                                         |
| Mixed string + component items           | Content appears in Markdown; components without tree render as plain text         |
| OrderedList with Layout                  | Layout has no effect on Markdown output (regression test)                         |
| Markdown equals MarkdownPlus             | Both methods produce identical output                                             |
| Item with inline styling (Prose)         | Styled text is degraded to plain text in both Markdown and MarkdownPlus           |

---

### `bt` CLI

- This specification will ensure that the **OrderedList** component:
    - has a 'bt' CLI subcommand for rendering this component
    - that the '--md' and '--html' CLI switches are available to render to Markdown and HTML targets respectively (the default render is always for the Terminal)
    - that the '--example' CLI switch is in place to provide a thoughtful example of how this command should be used with the CLI (see other working examples for a template)

#### Current State

| Aspect              | Status                                                           |
|---------------------|------------------------------------------------------------------|
| CLI command exists  | Partial — `bt list` exists but renders only `UnorderedList`      |
| Render method       | Bespoke — calls `UnorderedList::render()` directly               |
| Has `--md` switch   | No                                                               |
| Has `--html` switch | No                                                               |
| Has `--example`     | Yes (`bt list --example`)                                        |

The existing `bt list` command (`biscuit-terminal/cli/src/commands/list.rs`)
creates an `UnorderedList` with a configurable bullet (default `• `). It does
not support ordered lists, `--md`, or `--html` switches.

#### Specification Design

Extend the existing `bt list` command to support ordered list rendering via an
`--ordered` / `-o` flag. When this flag is set, the command creates an
`OrderedList` instead of a `UnorderedList`. Add `--md`, `--md-plus`, and
`--html` switches following the pattern established by `bt prose`.

**CLI structure:**

```
bt list [OPTIONS] [ITEMS]...

bt list --example
bt list "First item" "Second item" "Third item"
bt list --ordered "Step one" "Step two" "Step three"
bt list -o --md "First" "Second" "Third"
bt list -o --html "First" "Second" "Third"
bt list -o --example
```

**Updated args:**

| Flag                          | Type           | Description                                                       |
|-------------------------------|----------------|-------------------------------------------------------------------|
| `ITEMS`                       | `Vec<String>`  | Positional list items (required unless `--example`)               |
| `--example` / `-e`            | `bool`         | Render example and show command                                   |
| `--ordered` / `-o`            | `bool`         | Render as an ordered (numbered) list instead of unordered         |
| `--bullet` / `-b`             | `String`       | Bullet character for unordered lists (default: `• `)              |
| `--no-hanging-indent`         | `bool`         | Disable hanging indent on wrapped lines                           |
| `--html`                      | `bool`         | Render to HTML fragment (conflicts with `--md`, `--md-plus`)      |
| `--md`                        | `bool`         | Render to portable Markdown (conflicts with `--html`, `--md-plus`)|
| `--md-plus`                   | `bool`         | Render to MarkdownPlus (conflicts with `--html`, `--md`)          |
| `[command(flatten)]`          | `LayoutArgs`   | Shared margin/alignment flags                                     |

**Render path:**

1. Parse items (unescape shell escapes, wrap as Prose for styled text).
2. If `--ordered`:
   - Build `OrderedList::from(items)`.
3. Else:
   - Build `UnorderedList::from(items).with_bullet(&bullet)`.
   - Apply `--no-hanging-indent` if set.
4. Apply `LayoutArgs` to the component's layout.
5. **Terminal** (default): Render via `render(&term)`.
6. **HTML** (`--html`): Wrap in `BrowserTreeComponent` → `render_html_fragment()`.
7. **Markdown** (`--md`): Project tree → `render_markdown_node()`.
8. **MarkdownPlus** (`--md-plus`): Same as `--md` for lists (outputs are identical).

**Example definitions:**

```rust
// Unordered example (existing)
const LIST_EXAMPLE: &[&str] = &[
    "<b>Plan</b> the change",
    "<green>Run</green> focused tests",
    "Ship the smallest useful fix",
];
const LIST_EXAMPLE_CMD: &str = r#"bt list "<b>Plan</b> the change" "<green>Run</green> focused tests" "Ship the smallest useful fix""#;

// Ordered example (new)
const ORDERED_LIST_EXAMPLE: &[&str] = &[
    "Install dependencies",
    "Run the test suite",
    "Deploy to staging",
];
const ORDERED_LIST_EXAMPLE_CMD: &str = r#"bt list --ordered "Install dependencies" "Run the test suite" "Deploy to staging""#;
```

When `--example` is used with `--ordered`, render the ordered example and print
`ORDERED_LIST_EXAMPLE_CMD`. Otherwise, render the unordered example.

**Module changes**: Update `biscuit-terminal/cli/src/commands/list.rs` to import
`OrderedList` and handle the `--ordered` flag, plus the multi-target rendering
logic (following the pattern from `prose.rs`).

---

## Acceptance Criteria Summary

- [ ] `OrderedList`'s `TerminalRenderable::render()` delegates to the tree path by default
- [ ] Bespoke render path retained as `render_bespoke()` for parity testing
- [ ] `BrowserRenderable` achieved via `BrowserTreeComponent<OrderedList>`
- [ ] `MarkdownRenderable` implemented on `OrderedList` via tree renderer's Markdown path
- [ ] `bt list --ordered` / `bt list -o` renders an `OrderedList`
- [ ] `bt list -o --md` renders Markdown output (`1. First\n2. Second\n3. Third`)
- [ ] `bt list -o --html` renders HTML output (`<ol><li>...</li></ol>`)
- [ ] `bt list -o --md-plus` renders MarkdownPlus output (identical to `--md` for lists)
- [ ] `bt list -o --example` renders example with command display
- [ ] Parity tests (bespoke vs tree) cover all variants listed in Terminal IR section
- [ ] `KNOWN_DRIFT` ledger documents accepted divergences
- [ ] `bt list` (without `--ordered`) continues to work unchanged for `UnorderedList`

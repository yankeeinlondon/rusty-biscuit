---
last_updated: "2026-05-16"
---

# Challenges of Migrating the `OrderedList` Component to the Tree Rendering Architecture

## Functional and Design Goals

The `OrderedList` component exists to render a sequence of items as a **numbered list**
with correct prefix formatting (1., 2., 3., …), word-wrapping with hanging indentation so
continuation lines align with the start of the item text, and support for mixed content
types — plain strings, inline components (like `Prose`), and block-level components (like
nested lists or block quotes).

### Why OrderedList was created

Before `OrderedList`, emitting a numbered list required callers to manually format each
number prefix, manage per-item hanging indent widths (which vary — "1. " is 3 chars but
"10. " is 4), handle word wrapping that respects the prefix column, and recursively render
nested block content with appropriate indentation. `OrderedList` packages all of this into
a single `TerminalRenderable` component so that:

- Numeric prefixes are computed automatically from item position.
- Hanging indentation is computed per-item based on the actual prefix width (`visible_width`),
  so continuation lines align precisely after the number regardless of how many digits the
  number has.
- Inline components (e.g., `Prose`) receive automatic word-wrap configuration at `add()` time
  so their continuation lines also align correctly.
- Block-level children (e.g., nested lists, block quotes) are rendered without a number prefix
  and indented by a configurable `indent_children` amount (default: 4 spaces).
- The entire list participates in the `Layout` system (margins, alignment, word wrap) as a
  single block-level unit.

### Where OrderedList is used today

| Consumer | Crate | Usage pattern |
|----------|-------|---------------|
| `Compose::add_ordered_list()` | `biscuit-terminal` | Wraps an `OrderedList` into a `RenderableTerminalContent` as part of a composed output document |
| Terminal tree renderer (`render_list`) | `biscuit-terminal` | The `render_terminal_node` path for `NodeKind::List { ordered: true, .. }` creates an `OrderedList` and delegates to its `TerminalRenderable` impl |
| `bt list` command | `biscuit-terminal-cli` | The `bt` CLI's `list` subcommand renders arguments as an ordered list |
| Drift script | `scripts/drift.rs` | Markdown-to-component conversion creates `OrderedList` from detected ordered-list lines |

### Example usage

```rust
use biscuit_terminal::prelude::*;

// Create from a vec of strings
let list = OrderedList::new(vec!["First item", "Second item", "Third item"]);
let output = list.render_optimistic(None);
// "1. First item\n2. Second item\n3. Third item\n"

// Build incrementally with add()
let mut list = OrderedList::empty();
list.add("Install dependencies").add("Run build").add("Deploy");

// Mixed content: strings + inline components + nested block components
let items = vec![
    RenderableTerminalContent::String("Plain string".to_string()),
    RenderableTerminalContent::Component(Rc::new(Prose::new("Inline text"))),
    RenderableTerminalContent::Component(Rc::new(
        OrderedList::new(vec!["Nested A", "Nested B"]),
    )),
];
let list = OrderedList::from(items);
// Renders:
// 1. Plain string
// 2. Inline text
//     1. Nested A
//     2. Nested B

// Via Compose
let mut doc = Compose::default();
doc.add_text("Steps:\n")
    .add_ordered_list(OrderedList::new(vec!["Clone the repo", "Run cargo build"]));
let rendered = doc.render(&terminal);
```

## Technical Implementation (current)

### Structure

The component lives at `biscuit-terminal/lib/src/components/list.rs` and consists of:

- **`OrderedList`** — owns:
  - `items: Vec<RenderableTerminalContent>` — heterogeneous content items (strings, inline
    components, block-level components)
  - `layout: Layout` — standard layout configuration (margins, alignment, word wrap)
  - `indent_children: u32` — indentation for block-level children (default: 4)

### Rendering pipeline

`OrderedList` implements `TerminalRenderable`. The rendering flow is:

1. **`render_optimistic(term_width)` / `render(&term)`** — entry points. They compute the
   available width from the layout and delegate to `render_content`.
2. **`render_content(term, term_width)`** — private method that iterates over `self.items`
   with `enumerate()`, computing a per-item number prefix. For each item, three rendering
   branches exist:
   - **String items** — the prefix (e.g., "1. ") is prepended. The text is word-wrapped
     using `WrapProse(None, Some(prefix_width))` as the hanging indent, so continuation
     lines align after the number. The available content width is `term_width - prefix_width`.
   - **Block-level components** (`is_block_level() == true`) — no prefix is emitted. The
     component is rendered at `term_width - indent_children`. Each line of the component's
     output is indented by `indent_children` spaces.
   - **Inline components** — the prefix is prepended. The component is rendered at
     `term_width - prefix_width`. Its word wrap was configured at `add()` time (via
     `configure_component_wrap`) so it handles its own continuation alignment.
3. **`Layout::apply_block_layout`** — the rendered string is passed through the layout system
   to apply margins and alignment as a cohesive block, preserving the vertical number column.

### Helper functions

Two free functions support the inline-component hanging indent system:

- **`configure_component_wrap(content, hanging_indent)`** — sets `WordWrap::with_hanging_indent_if_none`
  on an inline component's layout, preserving any explicit value the caller already set.
- **`force_component_hanging_indent(content, hanging_indent)`** — unconditionally replaces
  the hanging indent, used when the bullet/prefix changes after initial construction
  (relevant for `UnorderedList::with_bullet`).

### Key responsibilities

| Responsibility | How it's handled |
|----------------|------------------|
| Number prefix generation | `format!("{}. ", number)` per item, computed from `enumerate()` index |
| Variable-width prefix | `visible_width(&prefix)` determines per-item hanging indent and available width |
| Hanging indentation | `WordWrap::WrapProse(None, Some(prefix_width))` for string items; `configure_component_wrap` for inline components |
| Mixed content rendering | Three-way match on `RenderableTerminalContent` variant + `is_block_level()` check |
| Nested block children | Rendered at reduced width, indented by `indent_children`, no prefix |
| Width propagation | `term_width.saturating_sub(prefix_width)` for inline, `term_width.saturating_sub(indent_children)` for block |
| Block-level behavior | `is_block_level() -> true` |
| Layout integration | Owns a `Layout`; full rendered content passes through `apply_block_layout` |

## Implementation Challenges

### Variable-Width Number Prefix

#### Challenge Description

Each item's number prefix has a different visible width: "1. " is 3 characters, "10. " is
4, "100. " is 5, and so on. The current bespoke renderer computes `visible_width(&prefix)`
per item and uses it for both hanging indent and available-width calculation. The tree
model's `NodeKind::ListItem` has no field for prefix width — it only carries `checked` and
`children`. When the tree renderer encounters a `ListItem`, it calls `render_blocks` on
the children and then prepends a number in `render_list`, but the **per-item width
variation** is lost because `OrderedList::from(items)` (which the tree renderer uses) treats
all items as `String` content and applies uniform formatting during its own render pass.

The challenge is that in the tree, all `ListItem` nodes are structurally identical — there is
no place to store "this item's prefix occupies N columns." A tree renderer that walks the
list items must either recompute the prefix width from the item's position (which the tree
does not track — items are just children of `List`) or accept uniform prefix treatment.

#### Example

```rust
// A 12-item ordered list: items 1–9 have prefix width 3 ("1. "),
// items 10–12 have prefix width 4 ("10. ").
let items: Vec<String> = (1..=12).map(|i| format!("Item {i}")).collect();
let list = OrderedList::new(items);
let output = list.render_optimistic(Some(20));
// Item 10's hanging indent must be 4 spaces, while items 1–9 use 3.
```

In the tree, all 12 items would be `ListItem` nodes with identical structure. A renderer
consuming the tree would need to infer the correct prefix width from the item's index
within the parent `List` node.

#### Suggested Test

```rust
#[test]
fn ordered_list_tree_honors_variable_prefix_width() {
    let items: Vec<String> = (1..=12).map(|i| format!("Item {i}")).collect();
    let list = OrderedList::new(items);
    let tree = list.render_tree();
    let term = Terminal::new_optimistic(20);
    let tree_output = render_terminal_node(
        &tree,
        &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
    )
    .expect("render")
    .output;
    let bespoke_output = list.render(&term);
    // Item 10 must have 4-space hanging indent, not 3.
    assert!(tree_output.contains("10. Item 10"),
        "tree output should number item 10 correctly");
    assert_eq!(
        strip_escape_codes(&tree_output),
        strip_escape_codes(&bespoke_output),
        "tree output should match bespoke rendering including prefix widths"
    );
}
```

### Heterogeneous Content Projection

#### Challenge Description

`OrderedList` stores its items as `Vec<RenderableTerminalContent>` — a mixed collection of
plain strings and `Rc<dyn TerminalRenderable>` trait objects. When projecting into the render
tree, each item must become a `ListItem` node containing structured children. But plain
strings have no `TreeRenderable` implementation, and trait objects (`dyn TerminalRenderable`)
cannot be downcast to `dyn TreeRenderable` without additional type infrastructure.

This is the same heterogeneous-content problem that `Section` faces, but `OrderedList` has an
additional wrinkle: its three-way rendering logic (string / inline component / block-level
component) means the projection must decide whether a `ListItem`'s children should be a flat
`Paragraph > Text` or a structural subtree from a child component's own `render_tree()`.

#### Example

```rust
let inner = OrderedList::new(vec!["Nested A", "Nested B"]);
let prose = Prose::new("Inline text");
let items = vec![
    RenderableTerminalContent::String("Plain string".to_string()),
    RenderableTerminalContent::Component(Rc::new(prose)),
    RenderableTerminalContent::Component(Rc::new(inner)),
];
let list = OrderedList::from(items);
// Projecting to tree: each item must become a ListItem,
// but the third item is a block-level OrderedList whose
// tree projection itself must be a List node.
```

#### Suggested Test

```rust
#[test]
fn ordered_list_tree_projects_mixed_content_as_list_items() {
    let inner = OrderedList::new(vec!["Nested A", "Nested B"]);
    let prose = Prose::new("Inline text");
    let items = vec![
        RenderableTerminalContent::String("Plain string".to_string()),
        RenderableTerminalContent::Component(Rc::new(prose)),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ];
    let list = OrderedList::from(items);
    let tree = list.render_tree();
    let json = serde_json::to_string(&tree).expect("serialize");
    assert!(json.contains("ListItem"), "tree should contain ListItem nodes");
}
```

### Per-Item Hanging Indent Configuration

#### Challenge Description

When items are added via `OrderedList::add()`, the method computes the prefix width for
that item's position and calls `configure_component_wrap()` to set the hanging indent on
inline components. This mutation happens **at construction time**, not at render time.

In the tree model, `ListItem` nodes do not carry hanging-indent metadata. A tree renderer
that walks `ListItem` children must somehow know the correct hanging indent for each item.
The current terminal tree renderer (`render_list`) renders each `ListItem` child into a
string first, then passes the string collection to `OrderedList::from(items)` — which treats
all items as `String` variants and applies its own wrapping logic. This works for simple
cases but means any inline component inside a `ListItem` is already flattened to text before
reaching `OrderedList`.

#### Example

```rust
let mut list = OrderedList::empty();
list.add("A long item that will wrap across multiple lines in a narrow terminal");
list.add("Another long item that also wraps");
let output = list.render_optimistic(Some(30));
// Continuation lines of item 1 must align after "1. " (3 chars),
// and item 2 after "2. " (also 3 chars).
// If item 10 existed, its continuation lines would align after "10. " (4 chars).
```

#### Suggested Test

```rust
#[test]
fn ordered_list_tree_wraps_with_correct_hanging_indent_per_item() {
    let mut list = OrderedList::empty();
    // Add 10 items so item 10 has a wider prefix.
    for i in 1..=10 {
        list.add(format!("Item {i} with enough text to force wrapping in narrow terminal"));
    }
    let tree = list.render_tree();
    let term = Terminal::new_optimistic(30);
    let tree_output = render_terminal_node(
        &tree,
        &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
    )
    .expect("render")
    .output;
    let lines: Vec<&str> = strip_escape_codes(&tree_output).lines().collect();
    // Find the continuation line(s) for item 10.
    let item10_continuations: Vec<&&str> = lines
        .iter()
        .filter(|l| l.starts_with("   ") && !l.starts_with("10."))
        .collect();
    // Item 10's continuations should have 4-space indent, not 3.
    // (This test will initially fail, documenting the gap.)
}
```

### Block-Level Children Without Prefix

#### Challenge Description

When a block-level component (like a nested `OrderedList`, `UnorderedList`, or `BlockQuote`)
appears as a list item, the current `OrderedList` renders it **without a number prefix** and
indents its output by `indent_children` spaces (default: 4). The item still "occupies" a
number in the sequence, but the number is not displayed.

In the tree model, all items are `ListItem` nodes — there is no structural distinction
between an item that gets a number prefix and one that doesn't. The tree renderer's current
`render_list` method renders every `ListItem` child into a string and collects them, then
passes them all to `OrderedList::from(items)` — which treats them all as `String` variants
and applies uniform numbering. A block-level child embedded inside a `ListItem` would need
to signal that it should not receive a prefix.

This challenge is closely related to the heterogeneous content challenge: the distinction
between block-level and inline items is determined by `is_block_level()` at render time,
but the tree has no way to express this.

#### Example

```rust
let inner = OrderedList::new(vec!["Nested A", "Nested B"]);
let items = vec![
    RenderableTerminalContent::String("First".to_string()),
    RenderableTerminalContent::Component(Rc::new(inner)),
];
let list = OrderedList::from(items);
// Renders:
// 1. First
//     1. Nested A
//     2. Nested B
// Note: no "2." prefix before the nested list; the nested list is indented instead.
```

#### Suggested Test

```rust
#[test]
fn ordered_list_tree_block_child_gets_no_number_prefix() {
    let inner = OrderedList::new(vec!["Nested A", "Nested B"]);
    let items = vec![
        RenderableTerminalContent::String("First".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ];
    let list = OrderedList::from(items);
    let tree = list.render_tree();
    let term = Terminal::new_optimistic(80);
    let tree_output = render_terminal_node(
        &tree,
        &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
    )
    .expect("render")
    .output;
    let bespoke_output = list.render(&term);
    assert_eq!(
        strip_escape_codes(&tree_output),
        strip_escape_codes(&bespoke_output),
        "block-level child should not receive a number prefix in tree output"
    );
}
```

### Recursive Nesting and Width Compounding

#### Challenge Description

`OrderedList` supports arbitrary nesting — a list item can itself be an `OrderedList` (or
`UnorderedList`, or any block-level component). At each nesting level, the available width
shrinks: the outer list subtracts `prefix_width` or `indent_children` from the terminal
width, and the inner list does the same from the already-reduced width.

The current bespoke renderer handles this naturally because each nested list independently
computes its available width from the constrained width passed to it. In the tree model,
however, width propagation is not modeled — `NodeKind::List` has no width or constraint
field. The tree renderer (`render_list`) renders each `ListItem` into a string using the
**root** terminal width, then passes those strings to `OrderedList` which applies its own
prefix and wrapping. For nested lists, the outer `OrderedList` subtracts `indent_children`
and re-renders the inner content, but if the inner content was already rendered at the root
width, it may exceed the reduced available width.

Deep nesting (3+ levels) can drive the available width to zero or near-zero, producing
degenerate output. The bespoke renderer handles this with `saturating_sub`, but the tree
renderer may produce lines that exceed the terminal width.

#### Example

```
Three-level nesting:
  outer.render_optimistic(80)
    → inner (width 80 - 4 = 76)
      → deep (width 76 - 4 = 72)
        → "Deep" rendered at width 72

Current output:
        1. Deep
```

#### Suggested Test

```rust
#[test]
fn ordered_list_tree_nested_output_respects_reduced_width() {
    let deep = OrderedList::new(vec!["Deep"]);
    let middle = OrderedList::from(vec![RenderableTerminalContent::Component(Rc::new(deep))]);
    let outer = OrderedList::from(vec![RenderableTerminalContent::Component(Rc::new(middle))]);
    let width = 40u32;
    let tree = outer.render_tree();
    let term = Terminal::new_optimistic(width);
    let tree_output = render_terminal_node(
        &tree,
        &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
    )
    .expect("render")
    .output;
    for line in tree_output.lines() {
        let vis = visible_width(strip_escape_codes(line).as_str());
        assert!(
            vis <= width,
            "nested line exceeds width {}: {:?} ({})",
            width,
            line,
            vis
        );
    }
}
```

### Layout Properties Have No Tree Representation

#### Challenge Description

`OrderedList` owns a `Layout` that controls margins, alignment, word-wrap policy, and
row-fill strategy. It also has `indent_children: u32`, which controls how deeply nested
block content is indented. The render tree's `RenderNode` model has `NodeAttrs` (id,
classes, extension data) and `SourceSpan` (provenance) but no first-class representation
for layout properties or indentation configuration.

When `OrderedList` is projected into the tree, these settings are silently lost. Re-rendering
the tree through the terminal renderer will produce output without the original margins,
alignment, indent_children, or wrap settings.

#### Example

```rust
let list = OrderedList::new(vec!["First", "Second"])
    .with_indent_children(8)
    .left_margin(Margin::Chars(4))
    .right_margin(Margin::Chars(2));
let tree = list.render_tree();
// The tree has no way to carry indent_children=8, Margin::Chars(4), or Margin::Chars(2).
// Re-rendering would use default indent_children=4 and no margins.
```

#### Suggested Test

```rust
#[test]
fn ordered_list_tree_roundtrip_preserves_layout_intent() {
    let inner = OrderedList::new(vec!["Nested"]);
    let items = vec![RenderableTerminalContent::Component(Rc::new(inner))];
    let list = OrderedList::from(items)
        .with_indent_children(8)
        .left_margin(Margin::Chars(4));
    let tree = list.render_tree();
    let term = Terminal::new_optimistic(80);
    let tree_output = render_terminal_node(
        &tree,
        &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
    )
    .expect("render")
    .output;
    let bespoke_output = list.render(&term);
    assert_eq!(
        strip_escape_codes(&tree_output),
        strip_escape_codes(&bespoke_output),
        "tree roundtrip should preserve layout including indent_children and margins"
    );
}
```

### Bidirectional Rendering Circularity

#### Challenge Description

The existing terminal tree renderer already handles `NodeKind::List { ordered: true, .. }`
by creating an `OrderedList` component and calling its `TerminalRenderable::render()`. If
`OrderedList` itself is then migrated to implement `TreeRenderable` (projecting into
`NodeKind::List`), the terminal renderer would call `OrderedList::render_tree()` →
`NodeKind::List` → terminal renderer → `OrderedList` → `render_tree()` → infinite
recursion.

This is the same circularity challenge that `Section` faces with `NodeKind::Heading`.

#### Example

```
OrderedList::render()
  → OrderedList::render_tree()         // new TreeRenderable impl
    → NodeKind::List { ordered: true, children: [...] }
      → render_terminal_node()
        → Writer::render_list()
          → OrderedList::from(items)   // creates another OrderedList!
            → list.render()            // infinite recursion
```

#### Suggested Test

```rust
#[test]
fn ordered_list_tree_terminal_render_does_not_recurse_infinitely() {
    let list = OrderedList::new(vec!["First", "Second", "Third"]);
    let tree = list.render_tree();
    let term = Terminal::new_optimistic(80);
    // This should complete without stack overflow.
    let result = render_terminal_node(
        &tree,
        &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
    );
    assert!(
        result.is_ok(),
        "terminal render of OrderedList tree should succeed without recursion"
    );
}
```

### Starting Number Other Than 1

#### Challenge Description

The tree model's `NodeKind::List` has a `start: Option<u64>` field, allowing an ordered
list to begin at any number. The current `OrderedList` component always numbers from 1.
The terminal tree renderer already handles this mismatch with a special code path:
when `start != 1`, it calls `render_ordered_from(start, &items)` which manually formats
`{index}. {item}` strings, bypassing the `OrderedList` component entirely.

If `OrderedList` implements `TreeRenderable`, the projection must decide what `start` value
to set. Since `OrderedList` always starts at 1, the projection would always produce
`start: None` (or `start: Some(1)`). This is correct for lists that originate as
`OrderedList` components, but the reverse direction (tree → terminal) already handles
non-1 starts without using `OrderedList`.

The asymmetry means the `TreeRenderable` impl for `OrderedList` cannot faithfully round-trip
a tree that had `start: Some(5)`.

#### Example

```rust
// From the tree renderer (render.rs:346-349):
let origin = start.unwrap_or(1);
if origin != 1 {
    return Ok(self.render_ordered_from(origin, &items));
}
// render_ordered_from manually formats "5. x\n6. y\n" without using OrderedList.
```

#### Suggested Test

```rust
#[test]
fn ordered_list_tree_projection_always_starts_at_one() {
    let list = OrderedList::new(vec!["A", "B"]);
    let tree = list.render_tree();
    // The projected tree should have start: None or start: Some(1).
    if let NodeKind::List { start, .. } = &tree.kind {
        assert!(
            start.is_none() || start == &Some(1),
            "OrderedList projection should always start at 1, got start: {start:?}"
        );
    }
}
```

### Inline Component Wrap Mutation at Add Time

#### Challenge Description

When an item is added via `OrderedList::add()`, the method calls `configure_component_wrap()`
which mutates the component's `Layout.word_wrap` to include the correct hanging indent for
that item's position. This is a **side effect during construction** — the component's layout
is modified before it is stored in the items vector.

In the tree model, this per-item layout mutation has no equivalent. `ListItem` nodes do not
carry word-wrap configuration. If a `Prose` component is added at position 9 (prefix "9. ",
width 3) and then the list grows to 10+ items, the `Prose` component at position 9 still has
hanging indent 3 — which is correct for that item. But in the tree, this per-item state is
lost, and a renderer would need to re-infer it from the item's position.

Additionally, the `force_component_hanging_indent` function (used by `UnorderedList` when
the bullet changes) demonstrates that component layouts can be mutated after construction.
The tree model has no mechanism for post-construction layout updates on child nodes.

#### Example

```rust
let mut list = OrderedList::empty();
// Item 1: prefix "1. " → hanging indent 3
list.add(Prose::new("A long text that wraps"));
// Item 9: prefix "9. " → hanging indent 3
for i in 2..=9 {
    list.add(format!("Item {i}"));
}
// Item 10: prefix "10. " → hanging indent 4
list.add(Prose::new("Another long text that wraps"));
// Each Prose has a different hanging indent based on when it was added.
```

#### Suggested Test

```rust
#[test]
fn ordered_list_tree_prose_items_maintain_correct_hanging_indent() {
    let mut list = OrderedList::empty();
    list.add(Prose::new("A long enough prose item to force wrapping at narrow width"));
    for i in 2..=9 {
        list.add(format!("Item {i}"));
    }
    list.add(Prose::new("Another long prose item at position ten forcing different indent"));
    let tree = list.render_tree();
    let term = Terminal::new_optimistic(30);
    let tree_output = render_terminal_node(
        &tree,
        &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
    )
    .expect("render")
    .output;
    let bespoke_output = list.render(&term);
    assert_eq!(
        strip_escape_codes(&tree_output),
        strip_escape_codes(&bespoke_output),
        "tree output should match bespoke wrapping including per-item hanging indents"
    );
}
```

### Empty Nested Lists Produce Numbering Gaps

#### Challenge Description

When a block-level child (like a nested `OrderedList`) is empty, it still occupies a
position in the parent list's numbering sequence but produces no visible output. The
existing test `test_empty_nested_list` demonstrates this: an empty inner list at position 2
causes the next string item to be numbered 3 instead of 2, producing "1. Before\n\n3. After\n".

In the tree model, an empty `List` node has no `ListItem` children and would produce no
output when rendered. But when that empty list is a child of a `ListItem` in a parent list,
the parent's numbering must still account for it. The tree has no concept of "an item that
occupies a number but renders nothing."

#### Example

```rust
let inner = OrderedList::new(Vec::<String>::new());
let items = vec![
    RenderableTerminalContent::String("Before".to_string()),
    RenderableTerminalContent::Component(Rc::new(inner)),
    RenderableTerminalContent::String("After".to_string()),
];
let list = OrderedList::from(items);
let output = list.render_optimistic(Some(80));
// "1. Before\n\n3. After\n" — note the gap (no "2.") and blank line
```

#### Suggested Test

```rust
#[test]
fn ordered_list_tree_preserves_numbering_gap_from_empty_nested_list() {
    let inner = OrderedList::new(Vec::<String>::new());
    let items = vec![
        RenderableTerminalContent::String("Before".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
        RenderableTerminalContent::String("After".to_string()),
    ];
    let list = OrderedList::from(items);
    let tree = list.render_tree();
    let term = Terminal::new_optimistic(80);
    let tree_output = render_terminal_node(
        &tree,
        &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
    )
    .expect("render")
    .output;
    let bespoke_output = list.render(&term);
    assert_eq!(
        strip_escape_codes(&tree_output),
        strip_escape_codes(&bespoke_output),
        "tree should preserve the numbering gap (1. Before, blank, 3. After)"
    );
}
```

## Solution Suggestions

### Index-Aware List Rendering in the Terminal Renderer

#### Solution Description

Modify the terminal tree renderer's `render_list` method to compute the number prefix width
from each item's index within the parent `List` node's children, rather than delegating to
`OrderedList::from(items)` which treats all items uniformly. This would look like:

```rust
fn render_list(&mut self, ordered: bool, start: Option<u64>, children: &[RenderNode]) -> Result<String, RenderError> {
    if !ordered {
        // UnorderedList path unchanged
    }
    let origin = start.unwrap_or(1);
    let mut list = OrderedList::empty();
    for (idx, child) in children.iter().enumerate() {
        let number = origin + idx as u64;
        let prefix = format!("{number}. ");
        let prefix_width = visible_width(&prefix);
        let content = self.render(child)?;
        // Add the rendered string with position-aware wrapping
        list.add_with_prefix_width(content, prefix_width);
    }
    Ok(list.render(&self.opts.context.terminal))
}
```

This avoids changing the `NodeKind::ListItem` structure and keeps the width computation in
the renderer where the index is known.

#### Challenges Addressed

- **Variable-Width Number Prefix** — the renderer knows the index and can compute the prefix
  width per item.
- **Per-Item Hanging Indent Configuration** — `add_with_prefix_width` would configure the
  correct hanging indent for each item.

#### Variant Solutions

- Add a `prefix_width: u32` field to `NodeKind::ListItem` so the tree carries the
  pre-computed width. This couples terminal-specific layout into the tree model but avoids
  re-computation.
- Store the item index in `NodeAttrs.data` as a namespaced key (e.g., `"list.index"`).

### Tree-Aware Content Enum

#### Solution Description

Introduce a `TreeContent` trait or a `to_tree_nodes()` method on `RenderableTerminalContent`
that projects each variant into `RenderNode` children:

- `String(s)` → `RenderNode::paragraph(vec![RenderNode::text(s)])`
- `Component(c)` → if `c` implements `TreeRenderable`, call `c.render_tree()` and wrap
  the result as a `ListItem` child; otherwise fall back to ANSI-stripped text (matching
  the `BlockQuote` lossy pattern).

This provides a uniform projection path for the heterogeneous items that `OrderedList`
(and `Section`, `UnorderedList`) store.

#### Challenges Addressed

- **Heterogeneous Content Projection** — provides a uniform projection for string and
  component variants.
- **Block-Level Children Without Prefix** — the projection can check `is_block_level()` and
  produce a `ListItem` without text children for block components, signaling to the renderer
  that no prefix is needed.

#### Variant Solutions

- Use `Any::downcast` on `Rc<dyn TerminalRenderable>` to check for `TreeRenderable`
  implementation, avoiding a new trait but requiring `'static` bounds.
- Add a dedicated `TreeRenderable::render_tree_items()` method that returns `Vec<RenderNode>`
  instead of a single node, allowing `OrderedList` to produce its `ListItem` children
  directly.

### Block/Inline Discrimination in ListItem Semantics

#### Solution Description

Establish a convention that a `ListItem` node whose children contain only block-level nodes
(e.g., a `List` child) signals "no number prefix" to the renderer, while a `ListItem` with
inline/text children receives a number prefix. The terminal renderer would inspect each
`ListItem`'s children and apply the same logic that `OrderedList::render_content` currently
uses with `is_block_level()`.

Alternatively, add a `block: bool` field to `NodeKind::ListItem` (mirroring `NodeKind::Html`)
so the tree explicitly marks which items should not receive a prefix.

#### Challenges Addressed

- **Block-Level Children Without Prefix** — explicit marking avoids ambiguity.
- **Empty Nested Lists Produce Numbering Gaps** — a `ListItem` marked as block-level would
  be rendered without a prefix even if it produces no visible output, preserving the
  numbering sequence.

#### Variant Solutions

- Use `NodeAttrs.classes` to mark block-level items (e.g., `classes: vec!["block-only"]`),
  keeping the tree model unchanged.
- Accept the loss and always number every `ListItem`, changing the rendering contract so
  block-level children always appear within a numbered item.

### NodeAttrs Extension for Layout Metadata

#### Solution Description

Extend `NodeAttrs.data` to carry layout hints as namespaced key-value pairs:

```rust
node.attrs.data.insert("list.indent-children", "4");
node.attrs.data.insert("layout.left-margin", "4");
node.attrs.data.insert("layout.right-margin", "2");
```

Renderers that understand these keys apply them; those that don't ignore them. This
preserves layout intent through the tree without adding terminal-specific types to the
`renderable` crate.

#### Challenges Addressed

- **Layout Properties Have No Tree Representation** — layout metadata travels alongside
  the structural tree.
- **Per-Item Hanging Indent Configuration** — could encode per-item indent via
  `"list-item.hanging-indent"` on individual `ListItem` nodes.

#### Variant Solutions

- Add a dedicated `Layout` field to `RenderNode` for type safety (couples layout types into
  `renderable`).
- Accept the loss and handle layout only at the `TreeComponent` adapter level, matching the
  current `BlockQuote` approach where layout is applied post-tree-render.

### Split Terminal and Tree Render Paths

#### Solution Description

Break the potential infinite recursion between `OrderedList`'s `TerminalRenderable` and the
terminal tree renderer's `NodeKind::List` handler by establishing a clear one-directional
relationship:

1. `OrderedList` implements `TreeRenderable` to project into `NodeKind::List`.
2. `OrderedList`'s `TerminalRenderable` impl **continues to use its bespoke rendering**
   (not the tree path) for terminal output.
3. The terminal tree renderer's `NodeKind::List` handler continues to create an
   `OrderedList` and call `render()` — this is fine because it is the *consumer* of the
   tree, not a re-entrant call.
4. The `TreeComponent<OrderedList>` adapter is used only when someone explicitly wants to
   route an `OrderedList` through the tree for Markdown or Browser output.

This matches the additive approach used for `BlockQuote`: the tree path is an additional
capability, not a replacement for the existing terminal rendering.

#### Challenges Addressed

- **Bidirectional Rendering Circularity** — no recursion; the terminal path and tree path
  are independent.

#### Variant Solutions

- Eventually flip `OrderedList::render()` to delegate through the tree (replacing bespoke
  code), but only after a parity gate proves the tree path produces identical output. This
  is the long-term goal described in `tree-rendering.md` step 4.

### Acceptable Start-Number Asymmetry

#### Solution Description

Formalize the current design: `OrderedList` always starts at 1 and always projects
`start: None` (or `start: Some(1)`) into the tree. Non-1 starts are handled by the tree
renderer's `render_ordered_from` path, which does not use `OrderedList` at all. This means:

- The `TreeRenderable` impl for `OrderedList` sets `start: None`.
- The terminal renderer's `NodeKind::List` handler checks `start` and branches accordingly
  (already implemented).
- Round-tripping a tree with `start: Some(5)` through `OrderedList` is not supported and
  is documented as out of scope.

This is the simplest approach and reflects the current behavior.

#### Challenges Addressed

- **Starting Number Other Than 1** — documents the asymmetry and makes it intentional
  rather than accidental.

#### Variant Solutions

- Add a `start: u64` field to `OrderedList` so it can represent any starting number,
  making the projection symmetric. This would require updating `OrderedList::new`,
  `OrderedList::add`, and the prefix-width computation to account for non-1 origins.

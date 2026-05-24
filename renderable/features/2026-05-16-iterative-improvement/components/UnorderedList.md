---
last_updated: "2026-05-16"
---

# Challenges of Migrating the `UnorderedList` Component to the Tree Rendering Architecture

## Functional and Design Goals

The `UnorderedList` component exists to provide a terminal-native bullet-point list with correct text wrapping, hanging indentation, and support for mixed content types (plain strings, inline components, and nested block-level components). It is one of the most frequently used structural components across the entire rusty-biscuit monorepo.

### Why It Was Created

1. **Consistent bullet rendering** — A reusable component ensures every bullet list in every CLI tool renders identically: same default bullet (`- `), same hanging indent, same width-aware wrapping.
2. **Hanging indent for wrapped lines** — When a list item wraps across multiple terminal lines, continuation lines must align with the start of the item text (after the bullet), not with the bullet itself. This is a typographic convention that greatly improves readability in narrow terminals.
3. **Mixed content support** — List items are not always plain strings. An item can be an inline component (e.g., a `Prose` with styled text and hyperlinks) or a block-level component (e.g., a nested `UnorderedList`, an `OrderedList`, or a `Table`). The component must handle all three uniformly.
4. **Customizable bullets** — Different call sites need different bullet characters (e.g., `-`, `→`, `•`, `◦`, or multi-character bullets like `  - ` with leading indentation). The component must support arbitrary bullet strings and recalculate hanging indents when the bullet changes.
5. **Width-aware rendering** — The component must respect terminal width, including margins and available width constraints, and never emit lines wider than the terminal.

### Where It Is Used Today

`UnorderedList` is one of the most widely used components across the monorepo. Major consumers include:

| Crate | Module / File | Usage |
|-------|---------------|-------|
| `sniff-cli` | `output/filesystem/mod.rs` | Repository metadata lists (branches, remotes, config, pagers, crypto) |
| `sniff-cli` | `output/filesystem/deps.rs` | Dependency listing with custom bullet `"  "` |
| `sniff-cli` | `output/filesystem/docs.rs` | Documentation file listings |
| `sniff-cli` | `output/filesystem/files.rs` | File tree listings |
| `sniff-cli` | `output/filesystem/repo.rs` | Repository info (tags, worktrees, commits, submodules) |
| `sniff-cli` | `output/filesystem/language.rs` | Language detection results with nested lists |
| `sniff-cli` | `output/commit_blocks.rs` | Git commit block rendering with custom bullet `"    - "` |
| `sniff-cli` | `output/just.rs` | Justfile recipe and variable listings |
| `sniff-cli` | `output/hardware.rs` | Hardware I/O device group listings |
| `sniff-cli` | `output/network.rs` | Network interface summaries |
| `claudine-cli` | `commands/skills.rs` | Skill catalog display with nested lists |
| `claudine-cli` | `commands/sync.rs` | Sync action display with custom bullet `"  ◦ "` and `"• "` |
| `claudine-cli` | `commands/link_display.rs` | Link metadata display |
| `claudine-cli` | `commands/logs/trends.rs` | Definition listings with custom bullet `"  "` |
| `claudine-cli` | `commands/logs/common.rs` | Log rendering |
| `claudine-cli` | `output/mod.rs` | Generic output formatting with bullet `"• "` |
| `homelab-cli` | `main.rs` | AV device command output |
| `biscuit-terminal` | `render_tree/render.rs` | Tree renderer delegates to `UnorderedList` for `List { ordered: false }` nodes |

### Example Usage

**Simple string list:**

```rust
let list = UnorderedList::new(vec!["Apple", "Banana", "Cherry"]);
// Renders:
// - Apple
// - Banana
// - Cherry
```

**Custom bullet with nested block-level children (from `sniff-cli` deps):**

```rust
let detail_list = UnorderedList::new(detail_items).with_bullet("  ");
let list = UnorderedList::from(outer_items).with_indent_children(Some(4));
```

**Incremental builder with Prose components (from `claudine-cli` skills):**

```rust
let mut outer_list = UnorderedList::empty();
outer_list.add(Prose::new(format!("<bold>{}</bold>", skill_name)));
let mut inner_list = UnorderedList::empty();
inner_list.add(Prose::new(skill_description));
outer_list.add(RenderableTerminalContent::Component(Rc::new(inner_list)));
```

**Custom bullet characters (from `claudine-cli` sync):**

```rust
let action_list = UnorderedList::from(action_proses).with_bullet("  ◦ ");
let mut list = UnorderedList::from(items).with_bullet("• ");
```

## Technical Implementation (current)

### Structure

`UnorderedList` is defined in `biscuit-terminal/lib/src/components/list.rs` alongside `OrderedList`. Both share the same architectural pattern but differ in prefix rendering (bullets vs. numbers).

```text
UnorderedList
├── items: Vec<RenderableTerminalContent>   // String | Component(Rc<dyn TerminalRenderable>)
├── bullet: String                          // default: "- "
├── hanging_indent: bool                    // default: true
├── layout: Layout                          // margins, alignment, row-fill, word-wrap
└── indent_children: Option<u32>            // block-level child indent; default: bullet width
```

`RenderableTerminalContent` is the polymorphic content type that enables mixed items: it is either a plain `String` or an `Rc<dyn TerminalRenderable>` pointing to any component.

### Rendering Pipeline

The `TerminalRenderable` implementation delegates to a private `render_content()` method that iterates each item and branches on its type:

```text
render() / render_optimistic()
  └── render_content(term, term_width)
        ├── String items:
        │     ├── prepend bullet
        │     ├── compute child_width = term_width - bullet_width
        │     ├── split_lines() → wrap_lines() with hanging indent
        │     └── join with "\n"
        ├── Block-level Component items:
        │     ├── no bullet prefix
        │     ├── compute child_width = term_width - indent
        │     ├── render child component in child_width
        │     └── indent every line by indent spaces
        └── Inline Component items:
              ├── prepend bullet
              ├── compute child_width = term_width - bullet_width
              ├── render child component in child_width
              └── (hanging indent was configured at add time)
  └── layout.apply_block_layout(content, width)
```

### Key Responsibilities

1. **Hanging indent management** — The component tracks a `hanging_indent` flag and, when enabled, configures inline child components' word-wrap policy at `add()` time via `configure_component_wrap()`. This sets `WrapProse(None, Some(indent))` on the child's `Layout` so the child handles its own continuation alignment.

2. **Bullet-width-aware wrapping** — String items are wrapped using `wrap_lines()` with `WordWrap::WrapProse(None, Some(indent))`, where `indent` equals the visible width of the bullet. This ensures continuation lines are padded to align after the bullet.

3. **Mixed content dispatch** — The three-way match on `RenderableTerminalContent` (String vs. block-level Component vs. inline Component) is the core branching logic. Block-level children are indented but not bulleted; inline children are bulleted.

4. **Custom bullet propagation** — `with_bullet()` detects when the new bullet has a different visible width than the old one and force-updates the hanging indent on all existing inline component items via `force_component_hanging_indent()`.

5. **Layout integration** — After rendering content, the result passes through `layout.apply_block_layout()`, which handles margins and block-level alignment (bullets form a vertical column that must be aligned as a unit).

6. **Width budget management** — `child_width` is computed differently for each item type: `term_width - bullet_width` for strings and inline components, `term_width - indent` for block-level children. Each nested level reduces the available width.

7. **Rc-based shared ownership** — Component items use `Rc<dyn TerminalRenderable>`, which means items cannot be mutated after insertion (unless there is a single owner). The `configure_component_wrap` and `force_component_hanging_indent` functions use `Rc::get_mut()` to attempt in-place mutation.

## Implementation Challenges

### Implementation Challenges

#### Challenge: Custom Bullet Representation

The tree's `List` node has an `ordered: bool` field and a `start: Option<u64>` for ordered lists, but **no field for custom bullet characters**. `UnorderedList` supports arbitrary bullet strings (e.g., `"→ "`, `"• "`, `"  ◦ "`, `"    - "`, `"  "`), and the tree renderer currently hardcodes the default `- ` bullet when constructing an `UnorderedList` from a `List` node.

**Example of the problem:** The `claudine-cli` sync command creates `UnorderedList::from(items).with_bullet("• ")`. If this component were to project itself into a `List { ordered: false, .. }` node, the custom bullet information would be lost. A downstream renderer consuming the tree would have no way to recover the original `"• "` bullet.

**Suggested unit test:**

```rust
#[test]
fn custom_bullet_survives_tree_roundtrip() {
    let original = UnorderedList::new(vec!["Alpha", "Beta"]).with_bullet("→ ");
    let tree = original.render_tree();
    let rendered = render_terminal_node(&tree, &TerminalRenderOptions::default()).unwrap();
    assert!(rendered.output.contains("→ Alpha"));
    assert!(rendered.output.contains("→ Beta"));
}
```

#### Challenge: Hanging Indent Semantics

The `UnorderedList` component manages hanging indentation (continuation lines align after the bullet) as a first-class concern. It has an explicit `hanging_indent: bool` flag, and when enabled, it configures word-wrap policies on child components at insertion time.

The tree model has no concept of hanging indent. `NodeKind::List` and `NodeKind::ListItem` carry no word-wrap metadata. Hanging indent is a terminal-rendering concern that is currently handled entirely within the bespoke `TerminalRenderable` impl.

**Example of the problem:** A long string item like `"This is a long item that wraps"` in a 20-column terminal should render as:

```text
- This is a long
  item that wraps
```

If the tree renderer processes a `List` → `ListItem` → `Text` subtree without hanging indent awareness, it would render:

```text
- This is a long
item that wraps
```

**Suggested unit test:**

```rust
#[test]
fn hanging_indent_preserved_in_tree_render() {
    let list = UnorderedList::new(vec!["This is a long item that wraps"]);
    let tree = list.render_tree();
    let rendered = render_terminal_node(&tree, &TerminalRenderOptions::default()).unwrap();
    let lines: Vec<&str> = rendered.output.lines().collect();
    assert!(lines.len() > 1, "Expected wrapping");
    assert!(
        lines[1].starts_with("  "),
        "Continuation should have hanging indent, got: {:?}",
        lines[1]
    );
}
```

#### Challenge: Mixed Content Type Discrimination

`UnorderedList` items can be plain strings, inline components (like `Prose`), or block-level components (like nested lists or tables). The current implementation branches on `RenderableTerminalContent`'s three variants and treats each differently: strings get inline wrapping, inline components get bullet + child-width rendering, and block-level components get indentation without bullets.

The tree's `ListItem` node has `children: Vec<RenderNode>` but no marker for whether the item's original content was a plain string, an inline component, or a block-level component. All content is projected into `RenderNode` variants (e.g., `Text`, `Paragraph`, `List`), and the distinction between "this was originally a Prose" vs. "this was originally a string" is lost.

**Example of the problem:** A list built with `list.add(Prose::new("<bold>Styled</bold>"))` and later read back from the tree would lose the information that the item was a `Prose` component with specific styling. The tree would contain `ListItem { children: [Text { value: "Styled" }] }` — the bold markup is flattened.

**Suggested unit test:**

```rust
#[test]
fn prose_component_survives_tree_projection() {
    let mut list = UnorderedList::empty();
    list.add(Prose::new("<bold>Important</bold> item"));
    let tree = list.render_tree();
    let rendered = render_terminal_node(&tree, &TerminalRenderOptions::default()).unwrap();
    // After projection, bold styling should still be present in terminal output
    assert!(rendered.output.contains("Important"));
}
```

#### Challenge: Block-Level Child Indentation

When a block-level component (like a nested `UnorderedList`) is placed inside a list, the parent list renders it without a bullet and indents all its output lines by `indent_children` spaces. This creates the visual nesting effect:

```text
- Top item
  - Nested A
  - Nested B
```

The tree model's `ListItem` node has no concept of "this is a block-level child that should not receive a bullet but should be indented." The terminal renderer would need to infer this from the structure — if a `ListItem` contains a `List` child, it should be treated differently — but this inference logic does not exist today.

**Example of the problem:** The tree renderer currently renders `List` items by first rendering each child into a string and then wrapping the strings in an `UnorderedList`. If a child is itself a `List` (block-level), it would be rendered and concatenated as a plain string item, losing the indentation distinction.

**Suggested unit test:**

```rust
#[test]
fn nested_list_indentation_in_tree_render() {
    let inner = UnorderedList::new(vec!["Sub A", "Sub B"]);
    let items = vec![
        RenderableTerminalContent::String("Top".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ];
    let list = UnorderedList::from(items);
    let tree = list.render_tree();
    let rendered = render_terminal_node(&tree, &TerminalRenderOptions::default()).unwrap();
    let lines: Vec<&str> = rendered.output.lines().collect();
    // The nested list items should be indented
    assert!(lines.iter().any(|l| l.starts_with("  - ")),
        "Expected indented nested bullet, got: {:?}", lines);
}
```

#### Challenge: Width Budget Compounding Across Nesting Levels

Each nesting level consumes width: the outer list subtracts its bullet width, the nested list subtracts its own bullet width, and so on. The current bespoke implementation tracks this through `child_width = term_width.saturating_sub(indent)` at each level, and the `indent_children` parameter controls how much width is reserved for block-level children.

The tree renderer delegates to the existing `UnorderedList` component for terminal rendering, which handles width compounding internally. But if a `TreeRenderable` impl on `UnorderedList` projects into the tree and a downstream renderer walks the tree, the width budget is not carried in the tree nodes. Each `ListItem` node has no width context. The renderer would need to compute cumulative indentation from the tree depth alone.

**Example of the problem:** Three levels of nested lists in a 40-column terminal:

```text
- Level 1
  - Level 2
    - Level 3 is a very long item that wraps
```

At level 3, only `40 - 2 - 2 - 2 = 34` columns remain for text. If the tree renderer does not track cumulative width reduction, the third-level item may not wrap correctly.

**Suggested unit test:**

```rust
#[test]
fn width_budget_compounds_across_three_levels() {
    let inner = UnorderedList::new(vec!["A very long item at depth three"]);
    let middle = UnorderedList::from(vec![
        RenderableTerminalContent::String("Depth two"),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ]);
    let outer = UnorderedList::from(vec![
        RenderableTerminalContent::String("Depth one"),
        RenderableTerminalContent::Component(Rc::new(middle)),
    ]);
    let tree = outer.render_tree();
    let rendered = render_terminal_node(&tree, &TerminalRenderOptions::default()).unwrap();
    for line in rendered.output.lines() {
        let width = visible_width(line);
        assert!(width <= 80, "Line exceeds terminal width: {:?} ({})", line, width);
    }
}
```

#### Challenge: Rc-Based Shared Ownership and Mutation

Items stored as `RenderableTerminalContent::Component(Rc<dyn TerminalRenderable>)` use reference-counted pointers. The `with_bullet()` method uses `Rc::get_mut()` to mutate items in-place when the bullet changes, but this only succeeds if there is a single owner. If items have been cloned (via the `From<Vec<&RenderableTerminalContent>>` impl), mutation silently fails.

This ownership model conflicts with the tree architecture, which requires owned `RenderNode` values. Projecting from `Rc<dyn TerminalRenderable>` into `RenderNode` either clones (losing the Rc connection) or consumes (preventing reuse of the original component).

**Example of the problem:** After calling `list.with_bullet("→ ")`, the internal `force_component_hanging_indent` tries `Rc::get_mut()` on each item. If another copy of the Rc exists (e.g., the same Prose was added to two different lists), the mutation is silently skipped, and the item keeps the old hanging indent.

**Suggested unit test:**

```rust
#[test]
fn bullet_change_updates_shared_items() {
    let prose = Prose::new("A long prose item that wraps at narrow widths");
    let shared_content: RenderableTerminalContent = prose.into();

    let mut list = UnorderedList::from(vec![shared_content.clone()]);
    list = list.with_bullet("→ ");  // wider bullet
    let result = list.render_optimistic(Some(20));
    let lines: Vec<&str> = result.lines().collect();
    // Continuation indent should match the new bullet width (2)
    for line in &lines[1..] {
        assert!(
            line.starts_with("  "),
            "Hanging indent should match bullet width: {:?}",
            line
        );
    }
}
```

#### Challenge: Prose Styling Loss During Projection

When a `Prose` component is placed in a list, its rich styling (bold, italic, colors, hyperlinks) is rendered through the `TerminalRenderable` trait. Projecting into the tree requires extracting the Prose's content into `RenderNode` variants (`Text`, `Emphasis`, `Strong`, `Link`, etc.), but the Prose component uses its own markup syntax (`<bold>`, `<italic>`, `<a href="...">`) which has no 1:1 mapping to the tree's `NodeKind` vocabulary for all features (e.g., Prose's `{{dim}}` token, custom color codes).

This is the same lossy-projection problem documented for `BlockQuote` in the tree-rendering.md parity tests, but it is amplified for lists because lists more frequently contain `Prose` items with complex styling.

**Example of the problem:** A `Prose::new("<bold><red>Error</red></bold>: file not found")` inside a list would need to be decomposed into `Strong → Text("Error")` + `Text(": file not found")`, but `<red>` has no `NodeKind` equivalent (color is not part of the tree vocabulary).

**Suggested unit test:**

```rust
#[test]
fn colored_prose_content_survives_projection() {
    let mut list = UnorderedList::empty();
    list.add(Prose::new("<bold><red>Error</red></bold>: file not found"));
    let tree = list.render_tree();
    let rendered = render_terminal_node(&tree, &TerminalRenderOptions::default()).unwrap();
    assert!(rendered.output.contains("Error"));
    assert!(rendered.output.contains("file not found"));
    // The bold/red styling may be lost — this test documents the loss
}
```

#### Challenge: Bidirectional Parity (Component-to-Tree-to-Terminal)

The tree rendering architecture requires that a component can project into a tree (`render_tree()`) and that a downstream renderer can produce the same terminal output as the bespoke `TerminalRenderable` impl. For `BlockQuote` this was proven with a parity test. For `UnorderedList`, the parity bar is much higher because of custom bullets, hanging indent, mixed content, and nested lists.

The current tree path for lists (in `render_tree/render.rs`) renders `List` nodes by delegating *back* to the existing `UnorderedList` component — creating a circular dependency. A true `TreeRenderable` impl on `UnorderedList` would need to project into the tree, and then the terminal renderer would need to render the tree *without* delegating back to `UnorderedList`, otherwise no migration has occurred.

**Example of the problem:** The current `render_list()` in the terminal renderer constructs an `UnorderedList::from(items)` where `items` are pre-rendered strings. This means the tree → terminal path is:

```text
tree List node → UnorderedList::from(strings) → TerminalRenderable::render()
```

If `UnorderedList` also implements `TreeRenderable`, the path becomes circular:

```text
UnorderedList::render_tree() → tree List node → render_list() → UnorderedList::from(strings) → ...
```

**Suggested unit test:**

```rust
#[test]
fn unordered_list_parity_between_bespoke_and_tree() {
    let term = Terminal::default();
    let mut list = UnorderedList::empty();
    list.add("Simple item");
    list.add(Prose::new("<bold>Styled item</bold>"));
    list.add("A longer item that should wrap when the terminal is narrow");

    let bespoke = list.render(&term);
    let tree = list.render_tree();
    let tree_result = render_terminal_node(&tree, &TerminalRenderOptions::default()).unwrap();

    // Content parity: both paths should contain the same text (after ANSI stripping)
    let bespoke_plain = strip_escape_codes(&bespoke);
    let tree_plain = strip_escape_codes(&tree_result.output);
    for item in &["Simple item", "Styled item", "longer item"] {
        assert!(bespoke_plain.contains(item));
        assert!(tree_plain.contains(item));
    }
}
```

#### Challenge: Disable Hanging Indent Flag

The `without_hanging_indent()` builder method disables hanging indent entirely, meaning wrapped lines are not indented at all. This flag has no representation in the tree model. If a list was created with `without_hanging_indent()`, projecting into the tree and rendering back would lose this configuration — the renderer would apply default hanging indent.

**Example of the problem:**

```rust
let list = UnorderedList::new(vec!["Short"]).without_hanging_indent();
// Bespoke: "- Short\n"
// Tree roundtrip might render with hanging indent configured
```

**Suggested unit test:**

```rust
#[test]
fn disabled_hanging_indent_survives_roundtrip() {
    let list = UnorderedList::new(vec!["A longer item that should wrap at narrow widths"])
        .without_hanging_indent();
    let result = list.render_optimistic(Some(20));
    // When hanging indent is disabled, the item is rendered without wrapping indent
    // The tree roundtrip must preserve this behavior
}
```

#### Challenge: Indent Children Configuration

The `indent_children: Option<u32>` field controls how many spaces are used to indent block-level children. When `None`, it defaults to the bullet width. Call sites like `sniff-cli` explicitly set `with_indent_children(Some(4))` to create wider indentation for nested content.

This configuration is not represented in the tree's `List` or `ListItem` nodes. A tree-based renderer would need to either hardcode the indent value or infer it from some other signal.

**Example of the problem:** In `sniff-cli` language detection:

```rust
let list = UnorderedList::from(items).with_indent_children(Some(4));
```

If this list projects into a tree, the `4`-space indent is lost. A downstream renderer would use the default indent (bullet width, typically 2), changing the visual output.

**Suggested unit test:**

```rust
#[test]
fn explicit_indent_children_preserved_in_tree() {
    let inner = UnorderedList::new(vec!["Nested"]);
    let items = vec![
        RenderableTerminalContent::String("Parent"),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ];
    let list = UnorderedList::from(items).with_indent_children(Some(4));
    let bespoke = list.render_optimistic(Some(80));
    let lines: Vec<&str> = bespoke.lines().collect();
    // The nested item should be indented by 4 spaces
    assert!(lines[1].starts_with("    "), "Expected 4-space indent, got: {:?}", lines[1]);
}
```

## Solution Suggestions

#### Extend NodeKind::List with Bullet and Indent Metadata

**Description:** Add an optional `bullet` field to `NodeKind::List` (or to `NodeKind::ListItem`) that carries the custom bullet string. Also add an optional `indent_children` field to `List` to carry the block-level child indentation. This makes the tree self-describing for these layout properties.

```rust
List {
    ordered: bool,
    start: Option<u64>,
    bullet: Option<String>,         // NEW: custom bullet (None = default "- ")
    indent_children: Option<u32>,   // NEW: block-child indent (None = bullet width)
    hanging_indent: bool,           // NEW: whether to apply hanging indent
    children: Vec<RenderNode>,
}
```

**Which challenges it helps with:**

- **Custom Bullet Representation** — The bullet string is carried directly in the node, so downstream renderers can use it.
- **Hanging Indent Semantics** — The `hanging_indent` flag tells renderers whether to apply continuation alignment.
- **Disable Hanging Indent Flag** — The flag is explicitly represented.
- **Indent Children Configuration** — The value is explicitly represented.
- **Block-Level Child Indentation** — The renderer knows how much to indent block children.

**Variant solutions:**

- Store bullet/indent metadata in `NodeAttrs` (extension data) rather than adding fields to `NodeKind`, keeping the core vocabulary small. This avoids breaking the exhaustive match in every renderer but makes the metadata opt-in and easier to overlook.
- Use a dedicated `ListStyle` struct that wraps these fields, reducing the parameter count on `List`.

#### Add a Render-Hint Attribute System to NodeAttrs

**Description:** Introduce a namespaced "render hint" system within `NodeAttrs.data` (the extension map) that components can use to pass target-specific layout hints through the tree. For example, a `terminal` namespace could carry `hanging_indent`, `indent_children`, and `bullet_width` hints that only the terminal renderer reads.

**Which challenges it helps with:**

- **Hanging Indent Semantics** — The terminal renderer reads `terminal.hanging_indent` from attrs.
- **Width Budget Compounding** — A `terminal.cumulative_indent` hint could be set by parent list nodes.
- **Custom Bullet Representation** — A `terminal.bullet` hint carries the bullet string without polluting the core `NodeKind`.
- **Prose Styling Loss** — A `terminal.prose_markup` hint could carry the original Prose markup string for lossless reconstruction.

**Variant solutions:**

- Instead of namespaced hints in `NodeAttrs.data`, use a separate sidecar map (`HashMap<NodeId, RenderHints>`) that lives alongside the tree, avoiding node mutation.
- Define a typed `TerminalRenderContext` that is threaded through the terminal renderer's recursion, allowing parent nodes to set context that children read.

#### Compute Cumulative Indent in the Terminal Renderer

**Description:** The terminal renderer tracks cumulative indentation as it recurses into nested `List` nodes. Each time it enters a `List`, it adds the list's bullet width (or `indent_children`) to a running indent counter. When rendering text, it subtracts this cumulative indent from the available width.

**Which challenges it helps with:**

- **Width Budget Compounding Across Nesting Levels** — The renderer explicitly tracks how much width has been consumed by ancestor lists.
- **Block-Level Child Indentation** — The indent is applied from the running counter rather than from per-node metadata.

**Variant solutions:**

- Encode the indent budget in a `TerminalRenderContext` struct passed through the recursion, rather than computing it on the fly.
- Pre-compute the maximum nesting depth and validate that content fits before rendering.

#### Break the Circular Dependency with a Native List Renderer

**Description:** Implement list rendering directly in the terminal tree renderer (`Writer::render_list`) instead of delegating to the `UnorderedList` component. The native renderer would handle bullet prepending, hanging indent, and block-child indentation by reading metadata from the tree nodes (or render hints).

**Which challenges it helps with:**

- **Bidirectional Parity** — The circular dependency is broken because the tree renderer no longer delegates back to the component.
- **Mixed Content Type Discrimination** — The native renderer can inspect `ListItem.children` to determine whether an item contains block-level nodes (like a nested `List`) or inline-only nodes, and apply different formatting accordingly.

**Variant solutions:**

- Keep the delegation to `UnorderedList` for now but construct it with metadata extracted from the tree (bullet, indent, etc.), then migrate to native rendering once parity is proven.
- Create a `TreeUnorderedList` adapter that wraps a tree `List` node and implements `TerminalRenderable`, isolating the rendering logic.

#### Preserve Prose Markup as a Fallback Payload

**Description:** When projecting a `Prose` component into the tree, store the original Prose markup string in a `NodeAttrs.data` entry (e.g., `terminal.prose_markup`). The terminal renderer checks for this entry and, if present, renders the item using `Prose::new(markup).render()` instead of walking the inline subtree. This trades tree purity for lossless terminal rendering.

**Which challenges it helps with:**

- **Prose Styling Loss During Projection** — Colors, custom styles, and other Prose-specific markup are preserved verbatim.
- **Mixed Content Type Discrimination** — The renderer knows an item was originally a Prose and handles it accordingly.

**Variant solutions:**

- Extend `NodeKind` with a `StyledText { markup: String, format: String }` variant that carries the original markup and its format (e.g., `"prose"`). This is more explicit but adds a new variant to the exhaustive match.
- Accept the styling loss for the tree path and document it as a known degradation, following the same approach used for `BlockQuote`'s `Prose` flattening.

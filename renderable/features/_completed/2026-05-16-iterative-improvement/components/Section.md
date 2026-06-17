---
last_updated: "2026-05-16"
---

# Challenges of Migrating the `Section` Component to the Tree Rendering Architecture

## Functional and Design Goals

The `Section` component exists to give CLI programs a structured way to emit **titled
content blocks** — a heading followed by one or more content items — where the heading
carries both a semantic level (h1–h6) and visual styling appropriate for the terminal.

### Why Section was created

Before `Section`, composing a heading + body required callers to manually concatenate a
styled heading string with body lines, manage ANSI escapes, and handle layout (margins,
word wrap, alignment) themselves. `Section` packages all of that into a single
`TerminalRenderable` component so that:

- The heading prefix (`#`, `##`, etc.) and styling (bold for h1–h3, italic for h4–h5,
  plain for h6) are derived automatically from the `HeadingLevel`.
- Content items — plain strings, `Prose`, or any `TerminalRenderable` component — can be
  pushed into the section without the caller worrying about rendering or ANSI management.
- The entire section (heading + content) participates in the `Layout` system (margins,
  alignment, word wrap) as a single block-level unit.

### Where Section is used today

| Consumer | Crate | Usage pattern |
|----------|-------|---------------|
| `Compose::add_heading()` | `biscuit-terminal` | Wraps a `Section` into a `RenderableTerminalContent` as part of a composed output document |
| Terminal tree renderer | `biscuit-terminal` | The `render_terminal_node` path for `NodeKind::Heading` creates a `Section` and delegates to its `TerminalRenderable` impl |
| `bt` CLI (diagrams, text) | `biscuit-terminal-cli` | Indirectly via `Compose` when rendering structured help output |

### Example usage

```rust
use biscuit_terminal::prelude::*;

// Standalone usage
let mut section = Section::new(HeadingLevel::h2, "Getting Started");
section
    .push("Welcome to the tutorial.")
    .push("Let's begin with installation.");
let output = section.render_optimistic(Some(80));
// output: "\x1b[1m## Getting Started\x1b[22m\nWelcome to the tutorial.\nLet's begin with installation."

// Via Compose
let mut doc = Compose::new();
doc.add_heading("Configuration", 3);
doc.add_text("Edit the TOML file to change settings.");
let rendered = doc.render(&terminal);
```

## Technical Implementation (current)

### Structure

The component lives at `biscuit-terminal/lib/src/components/section.rs` and consists of:

- **`HeadingLevel`** — a `Copy` enum with variants `h1` through `h6`, each exposing a
  `.level() -> u8` method.
- **`Section`** — owns:
  - `level: HeadingLevel` — the heading level
  - `title: String` — the heading text
  - `content: Vec<RenderableTerminalContent>` — heterogeneous body items
  - `layout: Layout` — standard layout configuration (margins, alignment, word wrap)

### Rendering pipeline

`Section` implements `TerminalRenderable`. The rendering flow is:

1. **`render_optimistic(term_width)` / `render(&term)`** — entry points. They compute the
   available width from the layout and delegate to `render_content`.
2. **`render_content(term, term_width)`** — private method that:
   - Selects the Markdown prefix and ANSI styling based on `HeadingLevel`:
     - h1–h3: bold (`\x1b[1m` / `\x1b[22m`)
     - h4–h5: italic (`\x1b[3m` / `\x1b[23m`)
     - h6: plain
   - Writes the styled heading (prefix + title + reset)
   - Iterates over `self.content` and renders each `RenderableTerminalContent`:
     - `String` variants are emitted verbatim
     - `Component` variants are rendered via their own `TerminalRenderable` impl, using
       `render_in_width` when a terminal is available, or `render_optimistic` otherwise
   - Each content item is separated by a newline; a single trailing newline is stripped
3. **`Layout::apply_layout`** — the resulting string is passed through the layout system
   to apply margins, alignment, and word wrap.

### Key responsibilities

| Responsibility | How it's handled |
|----------------|------------------|
| Heading prefix and semantic level | `HeadingLevel` enum → prefix string (`# `, `## `, etc.) |
| Heading visual styling | Inline ANSI escape selection based on level |
| Heterogeneous content | `Vec<RenderableTerminalContent>` — strings + trait objects |
| Width propagation | Available width is subtracted from layout; child components receive the constrained width |
| Block-level behavior | `is_block_level() -> true` |
| Layout integration | Owns a `Layout` and applies it to the full rendered output |

## Implementation Challenges

### Heterogeneous Content Projection

#### Challenge Description

The `Section` component stores its body as `Vec<RenderableTerminalContent>` — a mixed
collection of plain strings and `Rc<dyn TerminalRenderable>` trait objects. When
projecting into the render tree, each content item must be converted into a
`RenderNode`, but the `TreeRenderable` trait method `render_tree(&self) -> RenderNode`
can only be called on types that implement it. Plain strings have no such implementation,
and trait objects (`dyn TerminalRenderable`) cannot be downcast to `dyn TreeRenderable`
without additional type infrastructure.

This is a harder version of the same problem `BlockQuote` faced — `BlockQuote` has a
single content item, so it could get away with a deliberately lossy `plain_text()` method
that strips ANSI codes. `Section` has **N** items and each may be a completely different
component type.

#### Example

```rust
let mut section = Section::new(HeadingLevel::h2, "Report");
section.push("Plain text paragraph.");
section.push(Prose::new("{{bold}}Key finding:{{reset}} results are positive."));
section.push(BlockQuote::from("A notable quote"));
```

When projecting this `Section` into the tree, the three content items must become
`Paragraph > Text`, `Paragraph > Strong + Text`, and `BlockQuote > Paragraph > Text`
respectively — but the `Section` only sees `RenderableTerminalContent` variants.

#### Suggested Test

```rust
#[test]
fn section_tree_projects_mixed_content_as_distinct_nodes() {
    let mut section = Section::new(HeadingLevel::h2, "Report");
    section.push("Plain text paragraph.");
    section.push(Prose::new("{{bold}}Key finding:{{reset}} results are positive."));
    let tree = section.render_tree();
    // The tree should contain separate child nodes for each pushed item,
    // not a single flattened text blob.
    let children = tree.children();
    assert!(children.len() >= 2, "expected at least 2 content children, got {}", children.len());
}
```

### Inline Style Loss During Projection

#### Challenge Description

The current `BlockQuote::render_tree()` implementation demonstrated a pattern that is
deliberately lossy: it calls `plain_text()`, which renders the component optimistically
and then strips all ANSI escape sequences. If `Section` follows the same approach for its
`Prose` content items, all rich inline styling (bold, italic, color, hyperlinks) would be
flattened to plain text in the tree.

This matters because the tree is supposed to be the canonical representation from which
Markdown, HTML, and terminal output are all produced. A tree that has lost the distinction
between bold and plain text cannot produce semantically correct Markdown (`**bold**`) or
HTML (`<strong>bold</strong>`).

#### Example

```rust
let mut section = Section::new(HeadingLevel::h1, "Title");
section.push(Prose::new("This is <b>important</b> and <i>emphasized</i>."));

// Lossy projection would produce:
//   Paragraph > Text("This is important and emphasized.")
// Correct projection should produce:
//   Paragraph > [Text("This is "), Strong(Text("important")), Text(" and "), Emphasis(Text("emphasized")), Text(".")]
```

#### Suggested Test

```rust
#[test]
fn section_tree_preserves_inline_styling_from_prose() {
    let mut section = Section::new(HeadingLevel::h1, "Title");
    section.push(Prose::new("Normal <b>bold</b> <i>italic</i> text."));
    let tree = section.render_tree();
    let json = serde_json::to_string(&tree).expect("serialize");
    // The serialized tree should contain Strong and Emphasis node kinds,
    // not just flat Text.
    assert!(json.contains("Strong") || json.contains("Emphasis"),
        "tree should preserve inline style structure, got: {json}");
}
```

### Recursive Tree Production for Nested Components

#### Challenge Description

When a `Section`'s content includes other `TerminalRenderable` components (e.g.,
`BlockQuote`, `Table`, `OrderedList`), the tree projection must recursively produce
subtrees for those children. This requires every component that can appear as `Section`
content to implement `TreeRenderable`. Currently only `BlockQuote` has such an
implementation.

Without this, the projection would need to either:
- Fail for unsupported component types (producing `Unsupported` nodes), or
- Fall back to rendering the component to a terminal string and embedding it as a
  plain `Code` or `Text` node (losing all structure).

This challenge compounds with the heterogeneous content challenge: the `Section` doesn't
know at compile time which component types will be in its content vector.

#### Example

```rust
let mut section = Section::new(HeadingLevel::h2, "Details");
section.push(BlockQuote::from("A notable insight"));
// If OrderedList doesn't implement TreeRenderable, the tree projection
// would degrade this to plain text.
```

#### Suggested Test

```rust
#[test]
fn section_tree_recursively_projects_block_quote_child() {
    let mut section = Section::new(HeadingLevel::h2, "Details");
    section.push(BlockQuote::from("A notable insight"));
    let tree = section.render_tree();
    // The content subtree should contain a BlockQuote node, not just Text.
    let json = serde_json::to_string(&tree).expect("serialize");
    assert!(json.contains("BlockQuote"),
        "tree should contain a BlockQuote child, got: {json}");
}
```

### Layout Properties Have No Tree Representation

#### Challenge Description

`Section` owns a `Layout` that controls margins, alignment, word-wrap policy, and row-fill
strategy. The render tree's `RenderNode` model has `NodeAttrs` (id, classes, extension
data) and `SourceSpan` (provenance) but no first-class representation for layout
properties.

When `Section` is projected into the tree, these layout settings are silently lost. This
means that re-rendering the tree back through the terminal renderer will produce output
without the original margins, alignment, or wrap settings that the `Section` author
intended.

The existing `TreeComponent` adapter sidesteps this because it applies layout *after*
tree rendering (it owns its own `Layout`), but when the component is flipped to delegate
through the tree internally, the layout must be preserved somehow.

#### Example

```rust
let section = Section::new(HeadingLevel::h2, "Indented Section")
    .left_margin(Margin::Chars(4))
    .right_margin(Margin::Chars(2));
let tree = section.render_tree();
// The tree has no way to carry Margin::Chars(4) or Margin::Chars(2).
// Re-rendering this tree would produce left-aligned, no-margin output.
```

#### Suggested Test

```rust
#[test]
fn section_tree_roundtrip_preserves_layout_intent() {
    let section = Section::new(HeadingLevel::h2, "Indented")
        .left_margin(Margin::Chars(4));
    let tree = section.render_tree();
    // Rendering the tree back to terminal should preserve the 4-char indent.
    // This test will initially fail, documenting the gap.
    let term = Terminal::new_optimistic(80);
    let tree_output = render_terminal_node(&tree, &TerminalRenderOptions::new(&term, RenderStrictness::Warn))
        .expect("render").output;
    let bespoke_output = section.render(&term);
    assert_eq!(tree_output, bespoke_output,
        "tree roundtrip should preserve layout");
}
```

### Terminal-Aware vs Target-Agnostic Styling

#### Challenge Description

The `Section` component's heading styling is **terminal-specific**: it emits raw ANSI
escape codes (`\x1b[1m` for bold, `\x1b[3m` for italic). The render tree is supposed to
be target-agnostic — it should carry semantic structure, not terminal presentation codes.

The tree already has `Strong` and `Emphasis` node kinds that could represent the heading
styling semantically. The challenge is that `Section`'s current heading logic maps
`HeadingLevel` directly to ANSI codes, bypassing any structural representation. A
migration must decide: should a `NodeKind::Heading` with `HeadingDepth(1)` imply bold, or
should the heading's inline children include `Strong` wrappers? This is a design decision
that affects every renderer.

#### Example

A `Section` with `HeadingLevel::h1` and title `"Title"` currently produces:
```
\x1b[1m# Title\x1b[22m
```

The tree should produce:
```
Heading { depth: 1, children: [Text("Title")] }
```

The Markdown renderer should produce `# Title`, the browser renderer should produce
`<h1>Title</h1>`, and the terminal renderer should apply bold — but the tree itself
must not contain ANSI codes.

#### Suggested Test

```rust
#[test]
fn section_tree_heading_contains_no_ansi_escapes() {
    let section = Section::new(HeadingLevel::h1, "Title");
    let tree = section.render_tree();
    let json = serde_json::to_string(&tree).expect("serialize");
    assert!(!json.contains("\\x1b"),
        "tree representation should not contain ANSI escape sequences");
    assert!(!json.contains("\x1b"),
        "tree representation should not contain raw escape bytes");
}
```

### Bidirectional Heading Rendering Consistency

#### Challenge Description

There is a subtle circularity: the existing terminal tree renderer
(`render_terminal_node`) already handles `NodeKind::Heading` by creating a `Section`
component and calling its `TerminalRenderable::render()`. If `Section` itself is then
migrated to implement `TreeRenderable` (projecting into `NodeKind::Heading`), the
terminal renderer would call `Section::render_tree()` → `NodeKind::Heading` → terminal
renderer → `Section` → `render_tree()` → ...

Breaking this cycle requires careful design. The terminal renderer's `NodeKind::Heading`
handler must not re-create a `Section` if the tree came from a `Section`. Alternatively,
the `Section` component could skip the tree path in its `TerminalRenderable` impl and
only use the tree for the Markdown and Browser render paths.

#### Example

```
Section::render()
  → Section::render_tree()  // new TreeRenderable impl
    → NodeKind::Heading
      → render_terminal_node()
        → Section::new(level, markup)   // creates another Section!
          → section.render()            // infinite recursion
```

#### Suggested Test

```rust
#[test]
fn section_tree_terminal_render_does_not_recurse_infinitely() {
    let mut section = Section::new(HeadingLevel::h2, "Test");
    section.push("Content");
    let tree = section.render_tree();
    let term = Terminal::new_optimistic(80);
    // This should complete without stack overflow.
    let result = render_terminal_node(
        &tree,
        &TerminalRenderOptions::new(&term, RenderStrictness::Warn),
    );
    assert!(result.is_ok(), "terminal render of Section tree should succeed");
}
```

### Multi-Item Content Requires Structural Grouping

#### Challenge Description

When a `Section` has multiple content items, the tree must decide how to group them.
The `NodeKind::Heading` variant has a flat `children: Vec<RenderNode>` — all children are
siblings under the heading. In contrast, `Section` today renders each content item as an
independent line, meaning the heading's children are implicitly separate paragraphs (or
block elements).

The tree renderer for `NodeKind::Heading` currently calls `render_inline` on the heading
children (treating them as inline content) and passes the result to `Section::new(level,
markup)`. This means only a **single** inline blob reaches the `Section`. If the tree
projection from `Section` produces multiple block-level children (one per content item),
the round-trip through the tree would flatten them into a single inline string.

This mismatch between "heading children as inline content" (current tree renderer) and
"section content as block-level items" (current `Section` behavior) must be resolved.

#### Example

```
Section { h2, "Report", content: ["Paragraph 1.", "Paragraph 2."] }

Projected tree:
  Heading { depth: 2, children: [
    Paragraph([Text("Paragraph 1.")]),
    Paragraph([Text("Paragraph 2.")])
  ]}

But the current terminal renderer does:
  let markup = render_inline(children)?; // joins all children inline
  Section::new(level, markup)           // single string title
```

#### Suggested Test

```rust
#[test]
fn section_tree_with_multiple_content_items_does_not_flatten_to_inline() {
    let mut section = Section::new(HeadingLevel::h2, "Report");
    section.push("First paragraph.");
    section.push("Second paragraph.");
    let tree = section.render_tree();

    // The heading's children should not be a single Text node joining both paragraphs.
    if let NodeKind::Heading { children, .. } = &tree.kind {
        // Expect multiple children, or at least a structure that preserves the
        // separation between the two paragraphs.
        let text_children: Vec<_> = children.iter()
            .filter(|c| matches!(&c.kind, NodeKind::Text { .. }))
            .collect();
        assert!(
            text_children.len() < children.len() || children.len() > 1,
            "content items should not be flattened into a single text node"
        );
    }
}
```

## Solution Suggestions

### Tree-Aware Content Enum

#### Solution Description

Introduce a new enum or trait method that allows `RenderableTerminalContent` to
participate in tree projection. Instead of the current binary `String` vs `Component`
split, each variant would carry enough information to produce a `RenderNode`:

- `RenderableTerminalContent::String(s)` → `RenderNode::paragraph(vec![RenderNode::text(s)])`
- `RenderableTerminalContent::Component(c)` → call `c.render_tree()` if the component
  implements `TreeRenderable`, or fall back to a `RenderNode::paragraph` with ANSI-stripped
  text (matching the existing `BlockQuote` lossy pattern).

A `TreeContent` trait could formalize this:

```rust
trait TreeContent {
    fn render_tree_content(&self) -> Vec<RenderNode>;
}
```

This would be implemented by `RenderableTerminalContent` and by any type that can be
converted into it.

#### Challenges Addressed

- **Heterogeneous Content Projection** — provides a uniform projection path for both
  string and component variants.
- **Recursive Tree Production** — the `Component` branch can call `render_tree()` on the
  inner type if it implements `TreeRenderable`.

#### Variant Solutions

- Instead of a new trait, add a method directly on `RenderableTerminalContent`:
  `fn to_tree_nodes(&self) -> Vec<RenderNode>`.
- Use `Any::downcast` on the `Rc<dyn TerminalRenderable>` to check if the component also
  implements `TreeRenderable`, avoiding the need for a new trait but requiring `'static`
  bounds.

### Prose-to-Tree Structural Projection

#### Solution Description

Give `Prose` its own `TreeRenderable` implementation that produces structural inline
nodes (`Strong`, `Emphasis`, `Text`) rather than flattening to a single ANSI-styled
string. This would parse the Prose token grammar (`{{bold}}`, `<b>`, `**bold**`) and
emit the corresponding node kinds.

This directly addresses the **Inline Style Loss** challenge and is a prerequisite for
`Section`'s tree projection to be faithful. The `BlockQuote` parity test already
documents this loss as accepted; migrating `Section` would raise the bar.

#### Challenges Addressed

- **Inline Style Loss During Projection** — structural inline nodes preserve semantic
  meaning across all render targets.
- **Heterogeneous Content Projection** — when a `Prose` content item is projected, it
  produces a proper `Paragraph` with structured inline children instead of flat text.

#### Variant Solutions

- Create a dedicated `prose_to_tree()` free function instead of adding a `TreeRenderable`
  impl to `Prose`, if the projection logic is too coupled to the `Section` context.
- Use the existing Prose token parser to produce an intermediate representation that
  maps 1:1 to `NodeKind` variants.

### NodeAttrs Extension for Layout Metadata

#### Solution Description

Extend `NodeAttrs` (or add a new field to `RenderNode`) to carry layout hints as
namespaced extension data. For example:

```rust
node.attrs.data.insert("layout.left-margin", "4");
node.attrs.data.insert("layout.alignment", "center");
```

Renderers that understand these keys can apply them; those that don't simply ignore them.
This preserves layout intent through the tree without adding terminal-specific types to
the `renderable` crate.

#### Challenges Addressed

- **Layout Properties Have No Tree Representation** — layout metadata travels alongside
  the structural tree.

#### Variant Solutions

- Add dedicated `Layout` field to `RenderNode` instead of using the generic `data` map.
  This is more type-safe but couples layout types into `renderable`.
- Accept the loss and handle layout only at the `TreeComponent` adapter level, matching
  the current `BlockQuote` approach where layout is applied post-tree-render.

### Split Terminal and Tree Render Paths

#### Solution Description

Break the potential infinite recursion between `Section`'s `TerminalRenderable` and the
terminal tree renderer's `NodeKind::Heading` handler by establishing a clear
one-directional relationship:

1. `Section` implements `TreeRenderable` to project into `NodeKind::Heading`.
2. `Section`'s `TerminalRenderable` impl **continues to use its bespoke rendering** (not
   the tree path) for terminal output.
3. The terminal tree renderer's `NodeKind::Heading` handler continues to create a
   `Section` and call `render()` — this is fine because it's the *consumer* of the tree,
   not a re-entrant call.
4. The `TreeComponent<Section>` adapter is used only when someone explicitly wants to
   route a `Section` through the tree for Markdown or Browser output.

This matches the additive approach used for `BlockQuote`: the tree path is an additional
capability, not a replacement for the existing terminal rendering.

#### Challenges Addressed

- **Bidirectional Heading Rendering Consistency** — no recursion; the terminal path and
  tree path are independent.
- **Terminal-Aware vs Target-Agnostic Styling** — the tree carries semantic structure;
  the terminal renderer maps that to ANSI codes.

#### Variant Solutions

- Eventually flip `Section::render()` to delegate through the tree (replacing bespoke
  code), but only after a parity gate proves the tree path produces identical output.
  This is the long-term goal described in `tree-rendering.md` step 4.

### Heading Children as Block-Level Content

#### Solution Description

Modify the tree model or the terminal renderer's `NodeKind::Heading` handler to treat
heading children as **block-level** content rather than inline content. Instead of:

```rust
let markup = render_inline(children)?;
let section = Section::new(level, markup);
```

The renderer would produce a `Section`, set the heading, and then render each block child
independently:

```rust
let section = Section::new(level, heading_text);
for child in children {
    section.push(render_block(child)?);
}
```

This requires a convention about what `NodeKind::Heading` children represent — inline
(heading text only) vs. block (heading text + body content). One approach is to use the
**first** child as the heading text and all subsequent children as body content.

#### Challenges Addressed

- **Multi-Item Content Requires Structural Grouping** — block children map naturally to
  `Section::push()` items.
- **Bidirectional Heading Rendering Consistency** — the renderer no longer collapses
  everything into a single inline string.

#### Variant Solutions

- Introduce a new `NodeKind::Section` variant with explicit `heading` and `body` fields,
  separating heading text from body content at the tree level.
- Keep `NodeKind::Heading` as-is but add a convention: if a heading has both inline and
  block children, the first inline-only sequence is the heading text and everything after
  the first block child is body content.

---
last_updated: "2026-05-16"
source_documents:
  - renderable/docs/tree-rendering.md
  - renderable/features/2026-05-16-iterative-improvement/components/OrderedList.md
  - renderable/features/2026-05-16-iterative-improvement/components/Progress.md
  - renderable/features/2026-05-16-iterative-improvement/components/Section.md
  - renderable/features/2026-05-16-iterative-improvement/components/Table.md
  - renderable/features/2026-05-16-iterative-improvement/components/TwoColumn.md
  - renderable/features/2026-05-16-iterative-improvement/components/UnorderedList.md
  - renderable/features/2026-05-16-iterative-improvement/components/YamlBlock.md
components:
  - OrderedList
  - Progress
  - Section
  - Table
  - TwoColumn
  - UnorderedList
  - YamlBlock
---

# Group 1 Tree Rendering Component Specification

## Goals

This spec synthesizes the component-level migration analyses for `OrderedList`,
`Progress`, `Section`, `Table`, `TwoColumn`, `UnorderedList`, and `YamlBlock`.
The goal is to identify the shared changes needed in the Tree Rendering
architecture and propose an implementation sequence that matures the
architecture while keeping each component migration parity-gated.

Success for this group means:

- Each component can produce a valid `RenderNode` tree through `TreeRenderable`.
- Each migration has a component-specific bespoke-vs-tree parity test before any
  production renderer is flipped.
- Shared architectural additions are introduced once and reused by later
  components.
- Components with inherently visual or terminal-specific behavior still gain a
  useful semantic tree projection, even when their terminal renderer remains
  bespoke initially.

## Architecture Enhancements

### 1. Add a Tree Content Projection Layer

Most group 1 components own heterogeneous terminal content:

- `Section` stores many `RenderableTerminalContent` body items.
- `OrderedList` and `UnorderedList` store list items as strings or
  `Rc<dyn TerminalRenderable>` components.
- `TwoColumn` stores left and right content as `RenderableTerminalContent`.
- `Table` has styled headers and cells that may contain pre-rendered `Prose`
  output.

The current tree architecture has `TreeRenderable`, but the terminal component
content containers do not have a reliable way to ask "can this item become tree
nodes?" This produces the same problem in several components: either flatten
everything to ANSI-stripped text or attempt fragile downcasts from
`dyn TerminalRenderable`.

Add a small projection layer around terminal content, for example:

```rust
pub trait TreeContent {
    fn render_tree_content(&self) -> Vec<RenderNode>;
}
```

or an equivalent method on `RenderableTerminalContent`:

```rust
impl RenderableTerminalContent {
    pub fn to_tree_nodes(&self) -> Vec<RenderNode>;
}
```

The contract should be:

- `String` becomes a paragraph or text node according to the caller context.
- Components that support `TreeRenderable` produce their native subtree.
- Unsupported components produce a visible `Unsupported` node in `Strict`, or an
  ANSI-stripped fallback paragraph in `Warn` / `Lossy`.
- Projection loss is reported as a diagnostic, not hidden.

This is the most important shared prerequisite. Without it, every component
would invent its own heterogeneous-content conversion and the architecture would
diverge immediately.

### 2. Define a Typed Render Hint Convention

Many missing fields are not semantic document structure, but they are required
to preserve caller intent:

- list bullet text, hanging-indent flag, block-child indent
- component `Layout` margins, alignment, wrapping, and row fill
- progress glyphs and bar width
- table cursor alignment, striping, column constraints, drop behavior, and
  vertical alignment
- two-column width configuration, gap, and stacking threshold
- YAML code-block chrome and highlighting preferences

The tree already has `NodeAttrs.data`, but the migration needs a documented,
typed convention for how components write and renderers read these values. Add
a reserved namespace such as:

```text
renderable.layout.*
renderable.terminal.*
renderable.table.*
renderable.list.*
renderable.widget.*
renderable.code.*
```

This can remain serialized through `NodeAttrs.data`, but Rust code should use
typed helper structs rather than ad hoc string keys at call sites:

```rust
ListRenderHints {
    bullet: Option<String>,
    hanging_indent: Option<bool>,
    indent_children: Option<u32>,
}

LayoutHints {
    left_margin: Option<u32>,
    right_margin: Option<u32>,
    alignment: Option<AlignmentHint>,
    row_fill: Option<RowFillHint>,
    word_wrap: Option<WordWrapHint>,
}
```

Renderers that do not understand a hint ignore it. Renderers that do understand
it can apply it and should emit diagnostics if strictness requires lossless
rendering and a required hint is malformed.

This avoids growing `NodeKind` for every terminal-only configuration while still
making the migration testable.

### 3. Add Layout Transfer From Components to Tree Rendering

`Section`, both list types, `Progress`, `Table`, `TwoColumn`, and `YamlBlock`
all own a `Layout`. If `render_tree()` returns only semantic structure, caller
layout choices disappear when the component is wrapped in `TreeComponent` or
rendered through a tree renderer.

Add one of these explicit mechanisms:

```rust
pub trait TreeRenderable {
    fn render_tree(&self) -> RenderNode;
    fn tree_layout_hints(&self) -> Option<LayoutHints> { None }
}
```

or a companion trait:

```rust
pub trait TreeLayout {
    fn tree_layout_hints(&self) -> LayoutHints;
}
```

`TreeComponent::new(component)` should copy these hints into the wrapper's
layout context or attach them to the root node. This is necessary before
flipping terminal renderers, because the bespoke implementations currently apply
layout after rendering the raw component body.

### 4. Introduce a Width-Aware Terminal Render Context

Several components cannot be rendered correctly from a tree without carrying
available width through recursive rendering:

- nested lists compound indentation and available width at each depth
- `TwoColumn` must render each child against a per-column width
- `Table` must plan columns against the current terminal width
- `Progress` applies layout to a single-line block
- `Section` and `YamlBlock` need margins and wrapping to affect child width

The terminal renderer should move toward an immutable context that is forked for
children:

```rust
TerminalRenderContext {
    available_width: u32,
    indent: u32,
    layout: LayoutHints,
    strictness: RenderStrictness,
}
```

Parent nodes can reduce `available_width` before rendering children. This keeps
the tree width-independent while allowing terminal output to remain
width-correct.

### 5. Replace Recursive Component Delegation With Native Tree Rendering

The current terminal tree renderer delegates some structural nodes back to
terminal components:

- `NodeKind::Heading` creates `Section`
- `NodeKind::List` creates `OrderedList` or `UnorderedList`
- `NodeKind::Table` creates `Table`

That was reasonable for the first tree renderer, but it becomes circular once
those same components implement `TreeRenderable` and eventually delegate their
terminal rendering through the tree.

For this group, native tree rendering should be introduced for:

- headings/sections
- lists and list items
- tables, at least after a compatibility adapter proves the projection shape

The implementation can be staged. A component may first add `TreeRenderable`
while keeping its bespoke `TerminalRenderable`, but a component should not be
flipped until its corresponding `NodeKind` is rendered without calling back into
that component's `TerminalRenderable` implementation.

### 6. Clarify Section vs. Heading Semantics

`NodeKind::Heading` represents a heading's inline content today. `Section`
represents a heading plus block-level body content. Encoding a whole `Section`
as a `Heading` with mixed inline and block children overloads the existing
meaning and risks flattening multiple body items.

Prefer adding a first-class section representation:

```rust
NodeKind::Section {
    depth: HeadingDepth,
    heading: Vec<RenderNode>,
    children: Vec<RenderNode>,
}
```

If adding a new node kind is considered too heavy for this phase, establish a
temporary convention:

- a component `Section` projects as a `Root` containing a `Heading` followed by
  body block nodes
- parsed Markdown headings remain `NodeKind::Heading`

The first-class node is cleaner for component migration, but the temporary
convention avoids changing parsed Markdown semantics.

### 7. Extend List Semantics Without Overfitting

Lists are central to this group and should mature the core list model. Add a
typed `ListStyle` or render-hint payload that covers:

- `ordered`
- `start`
- custom bullet string for unordered lists
- `hanging_indent`
- `indent_children`

The terminal renderer should compute ordered prefix widths from the item index
and `start`, not store per-item prefix width in the tree. Prefix width is a
render-time consequence of the ordered list state.

The renderer must also distinguish inline-only list items from block-level list
items. A practical rule is:

- inline-only children receive the bullet or number prefix
- block-only children are indented by `indent_children` and do not receive an
  additional prefix
- mixed items render the first inline paragraph with the prefix, then render
  following block children indented

This matches current bespoke behavior while making nested lists and block quotes
work naturally.

### 8. Add General Widget Support or Targeted Nodes for Visual Components

`Progress` and `TwoColumn` are not pure document structure. They carry semantic
content, but their terminal rendering is visual and width-dependent. The tree
should support them without pretending they are ordinary paragraphs.

There are two reasonable paths:

1. Add targeted nodes:

```rust
NodeKind::Progress {
    value: f32,
    label: Option<String>,
}

NodeKind::Columns {
    gap: u32,
    children: Vec<RenderNode>,
}
```

2. Add a more general widget node:

```rust
NodeKind::Widget {
    widget_type: String,
    children: Vec<RenderNode>,
}
```

For group 1, targeted nodes are preferable if the team expects `Progress` and
`TwoColumn` to become common cross-target primitives. A widget node is preferable
if this is expected to remain a small escape hatch for terminal-first
components.

In either design, semantic data should live in typed fields where possible, and
terminal-only details such as progress glyphs or column cursor strategy should
live in render hints.

### 9. Add a Code-Block Highlighting Extension Point

`YamlBlock` can project cleanly to `NodeKind::Code { lang: Some("yaml"), ... }`,
but parity with the existing component depends on syntax highlighting and the
full code-block chrome used by darkmatter.

Add an optional code-rendering hook to terminal and browser tree render options:

```rust
trait CodeRenderer {
    fn render_terminal_code(&self, lang: Option<&str>, value: &str, hints: &NodeAttrs)
        -> Option<String>;

    fn render_browser_code(&self, lang: Option<&str>, value: &str, hints: &NodeAttrs)
        -> Option<BrowserFragment<Ready>>;
}
```

The default hook keeps the current plain tree renderer behavior. Darkmatter can
provide a hook backed by `CodeHighlighter`, `format_header_row`, and
`render_html_code_block` without moving syntect into `renderable`.

This preserves the dependency boundary while making `YamlBlock` a useful
tree-rendered component.

### 10. Support Two-Pass Rendering for Complex Nodes

The renderer architecture should explicitly permit a node-specific pre-pass.
`Table` cannot be correct with a naive single-pass output writer because it must
measure all cells, resolve widths, compute row heights, apply vertical
alignment, build borders, and patch striping resets.

The terminal renderer should provide native two-pass handling for:

- `NodeKind::Table`
- future `NodeKind::Columns` if auto-sizing or image overlay is included

This is still an exhaustive node renderer; it just means a node handler may
buffer and measure descendants before emitting output.

### 11. Expand Component Parity Infrastructure

Every group 1 migration should add tests in the style of the existing
`BlockQuote` component parity gate. The shared test helpers should include:

- ANSI stripping and semantic token assertions
- width-limited rendering helpers
- line width assertions using visible width
- strict/warn/lossy render assertions
- diagnostics assertions for accepted projection loss
- component-specific helpers for lists, code blocks, tables, and layout margins

The important migration rule remains: a component is flipped only after its
parity gate proves the tree path is faithful enough for that component's
documented contract.

## Components That Should Not Be Included

All seven proposed components should be included in group 1, but not all should
be flipped to tree-backed terminal rendering at the same time.

`TwoColumn` should be included as a semantic projection first, not as an early
terminal-renderer flip. It has terminal-image overlay behavior, terminal-app
specific cursor strategies, and responsive stacking. Those are valuable stress
tests for the architecture, but they are too terminal-specific to require full
tree terminal parity before the simpler structural components are proven.

`Progress` should also be included, but it should be treated as a semantic
widget initially. It is visual, but it has portable semantic data: label and
completion value. Terminal glyph parity can wait until the widget or progress
node model is settled.

`Table` should be included because the existing tree already has table nodes,
but it should be scheduled after sections and lists. Table will force a two-pass
renderer and richer table metadata; doing it before the shared hint and content
projection layers exist would cause avoidable churn.

`YamlBlock` should be included as a code-block migration, with the understanding
that syntax-highlighting parity requires a render hook rather than new core
dependencies in `renderable`.

## Implementation Sequence

### Phase 0: Shared Foundations

Implement these pieces before adopting new components:

- tree content projection for `RenderableTerminalContent`
- typed render-hint helpers over `NodeAttrs.data`
- layout transfer from components into tree rendering
- width-aware terminal render context
- parity helper improvements

This phase should not flip any component. It should prove the infrastructure
with narrow synthetic tests and keep the current public render paths unchanged.

What we learn:

- whether heterogeneous terminal content can be projected without fragile
  downcasts
- whether layout and render hints can be serialized, validated, and ignored
  cleanly by renderers that do not use them
- whether the terminal renderer can safely pass width constraints through nested
  render calls

Architecture additions used:

- Tree Content Projection Layer
- Typed Render Hint Convention
- Layout Transfer
- Width-Aware Terminal Render Context
- Component Parity Infrastructure

### Phase 1: Section

Start with `Section`.

`Section` is the simplest high-value structural component in this group. It
exercises heading semantics, heterogeneous content, layout preservation, and the
recursive projection of child components. It also exposes an important modeling
decision: whether a component section is a `Section` node or a `Heading`
followed by body blocks.

Implementation target:

- `Section::render_tree()` produces semantic heading and body structure with no
  ANSI escapes.
- String body content becomes paragraph/text nodes.
- Tree-capable child components project recursively.
- Unsupported child components become visible fallback nodes with diagnostics.
- Terminal parity initially asserts semantic equivalence and layout preservation
  for simple content.

What we learn:

- how to represent a heading plus block body without flattening content
- whether the content projection layer works for multiple child items
- whether component layout can survive a tree round trip

Architecture additions used:

- Tree Content Projection Layer
- Layout Transfer
- Section vs. Heading Semantics
- Native heading/section terminal rendering, or a temporary non-recursive bridge

### Phase 2: UnorderedList and OrderedList

Implement the two list components together, with `UnorderedList` first and
`OrderedList` immediately after.

`UnorderedList` should go first because its prefix is fixed per list, so it is
the best component for proving custom bullet preservation, hanging indent,
block-child indentation, and nested width propagation. `OrderedList` then adds
the variable-width prefix problem for item 10, item 100, and non-default start
values.

Implementation target:

- both components project to `NodeKind::List` with typed list hints
- custom unordered bullets survive tree rendering
- disabled hanging indent and explicit `indent_children` survive
- ordered prefix width is computed by the renderer from list index and start
- nested block children are indented and do not receive an extra prefix
- native terminal list rendering replaces component delegation before any flip

What we learn:

- whether render hints are sufficient for target-specific list style
- whether the terminal context correctly compounds width and indentation across
  nested tree nodes
- whether the renderer can infer inline vs. block list item behavior from node
  structure

Architecture additions used:

- Tree Content Projection Layer
- Typed List Hints
- Width-Aware Terminal Render Context
- Native Tree List Renderer
- Component Parity Infrastructure

### Phase 3: YamlBlock

Implement `YamlBlock` after sections and lists because it is structurally
simple, but cross-crate and renderer-extension heavy.

Implementation target:

- `YamlBlock::render_tree()` produces a `Code` node with `lang = "yaml"` and the
  validated YAML body.
- code-block hints preserve the desire for full fence chrome/header behavior.
- terminal and browser tree renderers can accept optional code-rendering hooks.
- darkmatter supplies hooks that preserve existing syntax highlighting and HTML
  code-block structure.
- empty YAML renders non-empty output with a language label.

What we learn:

- whether renderer extension points can preserve rich output without moving
  darkmatter-only dependencies into `renderable`
- whether code-block parity can be described as body parity plus optional
  highlighting/chrome parity
- whether cross-crate components can adopt `TreeRenderable` cleanly

Architecture additions used:

- Typed Render Hint Convention
- Layout Transfer
- Code-Block Highlighting Extension Point
- Code-block parity helpers

### Phase 4: Progress

Implement `Progress` as the first widget-style component.

Progress is intentionally smaller than `TwoColumn` and `Table`, so it is a good
place to decide whether the architecture wants targeted nodes or a generic
widget node. It carries portable semantics (`label`, `value`) plus
terminal-only presentation hints (`bar_width`, glyphs, brackets).

Implementation target:

- projection carries semantic value and label, not a pre-rendered bar string
- terminal rendering preserves value, label, width hint, and glyph hints
- Markdown degrades to useful text such as `Label 75%`
- browser renders an idiomatic progress representation or at least structured
  progress markup with value data
- bespoke terminal rendering remains the primary path until widget parity is
  proven

What we learn:

- how to represent visual-but-semantic components without polluting document
  structure
- how target renderers should degrade widgets
- whether terminal-only render hints can preserve visual customization

Architecture additions used:

- Targeted `Progress` Node or Generic `Widget`
- Typed Render Hint Convention
- Layout Transfer
- Component Parity Infrastructure

### Phase 5: TwoColumn

Implement `TwoColumn` as a semantic projection after `Progress`.

`TwoColumn` should not be an early terminal flip. Its image overlay and
terminal-app-specific cursor strategies are closer to an inherently visual
renderer than a normal document node. However, the left/right semantic
relationship is useful for Markdown and browser output, and it stress-tests
width-aware child rendering.

Implementation target:

- projection carries two child subtrees, gap, and left-width configuration
- terminal tree rendering works for text/prose-only columns
- terminal image columns remain bespoke or produce an explicit unsupported
  diagnostic in strict tree rendering
- Markdown can degrade to a two-column table
- browser can render a structural two-column container with data attributes or
  CSS-ready classes
- full terminal cursor overlay parity is deferred

What we learn:

- whether child render contexts can fork with different width constraints
- where the boundary should sit between tree-rendered layout and permanently
  bespoke terminal rendering
- how strictness should report unsupported visual terminal features

Architecture additions used:

- Targeted `Columns` Node or Generic `Widget`
- Width-Aware Terminal Render Context
- Typed Render Hint Convention
- Tree Content Projection Layer

### Phase 6: Table

Implement `Table` last in this group.

Table has the richest existing behavior: width planning, conditional column
visibility, typed cells, column dropping, row notes, striping, cursor alignment,
multi-line cells, vertical alignment, and border construction. It should reuse
the content projection, hint, layout, and width-context work from earlier
phases, then add table-specific two-pass rendering.

Implementation target:

- projection produces `Table`, `TableRow`, and `TableCell` nodes with column
  metadata hints
- typed cell values are either preserved as table metadata or pre-formatted with
  alignment metadata
- terminal renderer performs a two-pass table render from tree nodes
- narrow and wide widths produce expected column visibility and drop notes
- striping and cursor-positioning preferences survive as terminal hints
- Markdown and browser renderers ignore terminal-only hints but preserve table
  content and alignment where possible

What we learn:

- whether the tree renderer can support node-specific multi-pass layout without
  compromising the exhaustive `NodeKind` contract
- whether table metadata belongs in typed `NodeKind::Table` fields or in
  namespaced hints
- whether lossy projection of styled cell content is acceptable or whether
  `Prose` needs full structural projection before table can be flipped

Architecture additions used:

- Typed Render Hint Convention
- Layout Transfer
- Width-Aware Terminal Render Context
- Two-Pass Terminal Rendering
- Table-specific parity helpers

## Proposed Milestones

### Milestone A: Projection Infrastructure

Deliver:

- `RenderableTerminalContent` tree projection helper
- typed render-hint helper API
- layout transfer convention
- parity helper expansion

Exit criteria:

- synthetic tests prove strings, tree-capable components, unsupported
  components, layout hints, and diagnostics behave consistently.

### Milestone B: Structural Blocks

Deliver:

- `Section::render_tree()`
- `UnorderedList::render_tree()`
- `OrderedList::render_tree()`
- native heading/list terminal rendering

Exit criteria:

- parity tests cover layout, nested content, custom bullets, disabled hanging
  indent, ordered item 10 wrapping, and block children without extra prefixes.

### Milestone C: Code Blocks and Widgets

Deliver:

- `YamlBlock::render_tree()`
- optional code-rendering hooks
- `Progress::render_tree()`
- first widget or progress node design

Exit criteria:

- YAML body and language parity pass, including empty YAML.
- Progress preserves label, percentage, width hint, and custom glyphs in the
  terminal tree path, with useful Markdown/browser degradation.

### Milestone D: Layout Primitives

Deliver:

- `TwoColumn::render_tree()`
- text/prose-only terminal tree rendering for columns
- strict diagnostics for image-overlay cases

Exit criteria:

- left/right content survives in all targets.
- width-dependent terminal output changes correctly between narrow and wide
  terminals.
- image cases keep bespoke terminal behavior or report explicit unsupported
  diagnostics in the tree path.

### Milestone E: Tables

Deliver:

- `Table::render_tree()`
- table column metadata hints
- two-pass terminal table rendering

Exit criteria:

- parity tests cover width planning, conditional columns, drop notes, typed cell
  formatting, multi-line row heights, vertical alignment, striping reset
  behavior, cursor preference, layout margins, and row fill.

## Open Decisions

- Should `Section`, `Progress`, and `Columns` become first-class `NodeKind`
  variants, or should they use `Root`/`Heading`, `Span`, and `Table`/`Widget`
  conventions?
- Should table column metadata become a typed field on `NodeKind::Table`, or
  remain in namespaced hints?
- Should `Prose` get a real structural tree projection before `Table`, or is
  lossy plain-text projection acceptable for this group?
- Should `TreeRenderable` expose layout directly, or should layout transfer be
  implemented as a separate optional trait?
- How strict should component parity be for visual components? For `Progress`
  and `TwoColumn`, semantic parity may be the right initial bar; for lists and
  sections, terminal text/layout parity should be required before flipping.

## Recommended Initial Scope

Start by landing the shared projection, hint, layout, and parity infrastructure,
then implement `Section`, `UnorderedList`, and `OrderedList`. These three are
the best first wave because they are structural, widely useful, and force the
architecture to solve the recurring problems without taking on table width
planning or visual widget behavior too early.

After those are stable, add `YamlBlock` and `Progress` to prove extension hooks
and widget semantics. Then add `TwoColumn` as a semantic layout projection, and
finish with `Table` once the renderer can support two-pass node handling.

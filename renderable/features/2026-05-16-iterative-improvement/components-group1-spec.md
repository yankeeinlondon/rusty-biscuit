---
last_updated: "2026-05-16"
source_documents:
  - renderable/docs/tree-rendering.md
  - renderable/features/2026-05-16-iterative-improvement/components-group1-review.md
  - renderable/features/2026-05-16-iterative-improvement/the-fold.md
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

This spec synthesizes the component-level migration analyses for seven group 1
components: `OrderedList`, `Progress`, `Section`, `Table`, `TwoColumn`,
`UnorderedList`, and `YamlBlock`.

The goal is to identify the shared changes needed in the Tree Rendering
architecture and define an implementation sequence that is executable against
the current code in `renderable`, `biscuit-terminal`, and `darkmatter`.

Success for this group means:

- each component can produce a valid `RenderNode` tree through `TreeRenderable`
- each component has a structural snapshot test for its projected tree
- each component tree passes `validate()` with zero error-severity findings
- each migration has bespoke-vs-tree semantic and positional parity tests before
  any production renderer is flipped
- every flip that changes native tree rendering also keeps the darkmatter Flow A
  parity gate green
- visual or terminal-specific components still gain useful semantic projections,
  even when their terminal renderers remain bespoke initially

## Settled Decisions

These decisions block implementation and should be treated as Phase 0 inputs,
not deferred open questions.

### `dyn TerminalRenderable` Projection

Use an optional tree method on `TerminalRenderable`:

```rust
pub trait TerminalRenderable {
    fn render_tree_node(&self) -> Option<RenderNode> {
        None
    }
}
```

The exact method name can be adjusted during implementation to avoid confusion
with `TreeRenderable::render_tree()`, but the mechanism should be this optional
method. This avoids downcasting and avoids replacing
`RenderableTerminalContent::Component(Rc<dyn TerminalRenderable>)` with a new
trait object type.

Projection behavior:

- `String` content becomes context-appropriate `Text` or `Paragraph` nodes.
- `Component(c)` calls `c.render_tree_node()`.
- If the component returns `Some(node)`, that node is inserted into the parent
  projection.
- If it returns `None`, the projection emits `Unsupported` in `Strict` or an
  ANSI-stripped fallback paragraph plus a diagnostic in `Warn` / `Lossy`.

The projection layer must carry a recursion depth guard. The default limit
should be conservative, configurable in tests, and reported as a diagnostic on
overflow. This prevents recursive component graphs from recursing indefinitely.

`Rc<dyn TerminalRenderable>` remains `!Send`/`!Sync`, but projected
`RenderNode` trees are owned and serializable. Once projected, the tree can
cross thread boundaries even when the source components cannot.

### Section Representation

Add a first-class section node:

```rust
NodeKind::Section {
    depth: HeadingDepth,
    heading: Vec<RenderNode>,
    children: Vec<RenderNode>,
}
```

`Section` is genuine document structure: a heading plus block-level body
content. Overloading `NodeKind::Heading` with body children would blur parsed
Markdown semantics and risks flattening multi-item component content.

Parsed Markdown can continue to produce `Heading` nodes followed by sibling
blocks. Renderers should support both `Heading` and `Section`.

Adding this node means updating all exhaustive `NodeKind` matches, serde JSON
snapshots, renderer tests, and validation rules in the same change.

### Progress and TwoColumn Representation

Do not add `NodeKind::Progress` or `NodeKind::Columns` for group 1.

Represent both as ordinary structural nodes with typed widget hints:

- `Progress` projects to a block-level `Paragraph` containing visible fallback
  text such as `Label 75%`, with `renderable.widget.progress.*` hints carrying
  semantic value and terminal glyph configuration.
- `TwoColumn` projects to a `Table`-like or generic block container shape that
  preserves left and right content, with `renderable.widget.columns.*` hints
  carrying gap, width, and stacking preferences.

This keeps the core `NodeKind` surface smaller. If more widgets later need
first-class handling, prefer one generic `NodeKind::Widget { widget_type,
children }` over many targeted variants.

### Table Cell Representation

Table cells should project readable text while preserving typed metadata:

- format `TableCellContent` into a visible `Text` node during projection
- record the original cell kind and value in `renderable.table.cell.*` hints
- record alignment and vertical alignment hints on `TableCell`

Markdown and browser renderers get readable table output from the text. The
terminal renderer can use the metadata for numeric and currency alignment.

### Browser Adapter

Phase 0 must add a browser tree adapter equivalent to terminal `TreeComponent`.
Because `BrowserRenderable::render_html_fragment()` is infallible while tree
rendering returns `Result`, the adapter should render with `Warn` semantics by
default:

- structural render errors produce a visible unsupported fragment and a
  diagnostic path where available
- non-fatal losses become diagnostics
- callers that need strict behavior should use the lower-level fallible tree
  browser renderer directly

### Code Rendering Hook

Code rendering hooks belong in tree render options and must be reachable through
`TreeComponent`. `TreeComponent` should expose a way to carry terminal render
options or hook configuration into `render_terminal_node`. Without this,
`YamlBlock` cannot preserve highlighting when rendered through the adapter.

### Provenance of Projected Nodes

Every `RenderNode` carries a `SourceSpan` with a `Provenance`. Projected
component nodes have no byte range in any text source, so the projection layer
must assign provenance deliberately rather than leaving it implicit.

For group 1, projected component nodes use `SourceSpan::synthetic()`
(`Provenance::Synthetic`, `location: None`). This matches how builder-created
nodes are already marked and keeps a projected component subtree internally
consistent.

The reason to settle this in group 1, rather than during a later renderer
migration, is that a projected component subtree is exactly the kind of subtree
a document-level pipeline would eventually splice into a larger parsed tree.
Fixing the convention now means a projected subtree drops into a mixed
`Provenance::Parsed` / `Synthetic` document without a later reclassification
pass. Structural snapshot tests serialize the `SourceSpan`, so the convention
is enforced from Phase 0.

## Architecture Enhancements

### 1. Tree Content Projection Layer

Build the projection layer around the optional method on `TerminalRenderable`
and a projection context:

```rust
TreeProjectionContext {
    strictness: RenderStrictness,
    max_depth: usize,
    current_depth: usize,
}
```

`RenderableTerminalContent` should expose a helper similar to:

```rust
fn to_tree_nodes(&self, context: &TreeProjectionContext) -> ProjectionResult<Vec<RenderNode>>;
```

The result should include diagnostics, because projection can lose styling,
encounter unsupported components, or hit the recursion limit before any renderer
runs.

`ProjectionResult` diagnostics must be the existing `renderable::tree::Diagnostic`
type — the same type tree validation and any document-level fold already
produce. Projection uses `DiagnosticKind::Unsupported` for unprojectable
components and recursion overflow, and `DiagnosticKind::Lossy` for accepted
styling loss. Introducing a projection-specific diagnostic type is explicitly
rejected: a single diagnostic vocabulary keeps component projection, tree
validation, and any future document-level pipeline mergeable without a
translation layer.

This layer is required by `Section`, both list components, `TwoColumn`, and the
styled-content parts of `Table`.

### 2. Typed Render Hints

Keep using `NodeAttrs.data` for extension data, but stop writing ad hoc keys at
component sites. Add typed helpers over reserved namespaces:

```text
renderable.layout.*
renderable.list.*
renderable.table.*
renderable.widget.progress.*
renderable.widget.columns.*
renderable.code.*
renderable.terminal.*
```

Initial hint shapes:

```rust
ListRenderHints {
    bullet: Option<String>,
    hanging_indent: Option<bool>,
    indent_children: Option<u32>,
}

LayoutHints {
    left_margin: Option<u32>,
    right_margin: Option<u32>,
    alignment: Option<String>,
    row_fill: Option<String>,
    word_wrap: Option<String>,
}

ProgressHints {
    value: f32,
    bar_width: Option<u32>,
    fill_char: Option<char>,
    empty_char: Option<char>,
    left_bracket: Option<char>,
    right_bracket: Option<char>,
}
```

Malformed required hints are diagnostics in `Warn` and errors in `Strict`.
Renderers that do not understand a namespace ignore it.

The typed helper layer must not hard-code the `renderable` prefix. Expose it as
a reusable pattern — generic over a namespace root — so other crates can define
their own typed helpers over their own reserved namespace, for example to carry
parser data that has no `NodeKind` field. `NodeAttrs.data` is a
`BTreeMap<String, serde_json::Value>`; the helper layer is the typed,
collision-resistant way to read and write it, and that contract should be the
same whether the writer is a component projection or a document-level pipeline.

### 3. Layout Transfer Through Existing `TreeComponent`

`TreeComponent` already owns a `Layout`. The gap is that it does not populate
that layout from the wrapped component. Add a default method to `TreeRenderable`
rather than a separate trait:

```rust
pub trait TreeRenderable {
    fn render_tree(&self) -> RenderNode;

    fn tree_layout_hints(&self) -> Option<LayoutHints> {
        None
    }
}
```

This is a trait-surface change, but the default method keeps existing impls
compatible. `TreeComponent::new(inner)` should read this method and initialize
its own layout or attach equivalent layout hints to the root node.

Nested tree rendering should then apply layout hints through the existing
terminal render context rather than losing them after projection.

### 4. Extend the Existing Terminal Render Context

`TerminalRenderContext` already exists in `biscuit-terminal`. Extend it rather
than introducing a new context type.

Required additions:

- distinguish root `width` from current `available_width`, or rename the field
  once call sites are clear
- track current indentation
- carry active layout hints
- provide fork helpers for child rendering

Example shape:

```rust
impl TerminalRenderContext {
    fn for_child(&self, indent_delta: u32, width_delta: u32) -> Self;
    fn with_width(&self, available_width: u32) -> Self;
    fn with_layout(&self, layout: LayoutHints) -> Self;
}
```

This is necessary for nested lists, sections with margins, two-column child
widths, and table width planning.

### 5. Replace Recursive Component Delegation With Native Tree Rendering

The terminal tree renderer currently delegates structural nodes back to
components, including `Heading`, `List`, `Table`, and `BlockQuote`. That
delegation makes early tree output possible, but it becomes circular once those
components implement tree-backed terminal rendering.

For group 1, native rendering is a hard ordering gate:

- `Section` parity does not count until `Heading` / `Section` tree rendering no
  longer calls `Section::render()`.
- list parity does not count until `List` rendering no longer calls
  `OrderedList::render()` or `UnorderedList::render()`.
- table parity does not count until `Table` rendering no longer calls
  `Table::render()`.

Temporary bridges are acceptable while developing, but they are not accepted as
parity gates and should not be the final state of a phase.

Every native-renderer change must keep darkmatter Flow A parity green because
parsed Markdown documents use the same terminal tree renderer.

### 6. List Semantics

Lists should continue to use `NodeKind::List` and `NodeKind::ListItem`.
The existing `ListItem { checked: Option<bool>, children }` field must be
preserved and tested; list style hints must not collide with task-list state.

List hints:

```text
renderable.list.bullet = string | null
renderable.list.hanging_indent = bool
renderable.list.indent_children = number | null
```

Rules:

- ordered prefix width is computed from list index and `start`
- unordered bullet width is computed from the hinted bullet or default bullet
- inline-only list items receive the bullet or number prefix
- block-only list items are indented by `indent_children` and do not receive an
  additional prefix
- mixed items render the first inline paragraph with the prefix, then render
  following block children indented
- checked items preserve `checked` and render consistently with existing
  task-list behavior

### 7. Code-Block Rendering Hook

`YamlBlock` projects naturally to `NodeKind::Code { lang: Some("yaml"), ... }`,
but parity with existing behavior needs darkmatter's highlighter and code-block
chrome.

Add optional hooks to terminal and browser render options:

```rust
trait CodeRenderer {
    fn render_terminal_code(
        &self,
        lang: Option<&str>,
        value: &str,
        attrs: &NodeAttrs,
        context: &TerminalRenderContext,
    ) -> Option<String>;

    fn render_browser_code(
        &self,
        lang: Option<&str>,
        value: &str,
        attrs: &NodeAttrs,
    ) -> Option<BrowserFragment<Ready>>;
}
```

Default renderers keep current plain behavior. Darkmatter can provide hooks
using `CodeHighlighter`, `format_header_row`, `render_terminal_code_block`, and
`render_html_code_block` without moving syntect into `renderable`.

`TreeComponent` must be able to pass these hooks into `render_terminal_node`;
otherwise `YamlBlock` rendered through the adapter will lose highlighting.

### 8. Two-Pass Rendering for Complex Nodes

The exhaustive `NodeKind` match contract allows a node handler to buffer and
measure descendants before emitting output. Two-pass handling is still one
explicit match arm; it is not a default-recursing visitor.

`NodeKind::Table` needs this. It must pre-scan rows and cells to resolve widths,
row heights, vertical alignment, border widths, dropped columns, and striping
state before emission.

Future widget-like column rendering may also need a two-pass path for auto
sizing, but group 1 should keep `TwoColumn` terminal parity limited to
text/prose-only cases.

### 9. Component Parity and Regression Infrastructure

All group 1 parity tests are Level 1 tests. They should use
`Terminal::new_optimistic(width)` and should not require a real terminal or PTY.

Shared helpers belong next to the current component parity test in
`biscuit-terminal/lib/tests/`, so phases extend one helper module instead of
copying normalization and width logic.

Every component must have four tiers:

1. Structural snapshot: serialize `render_tree()` to JSON and snapshot it.
2. Tree validity: run `validate()` and assert zero error-severity findings.
3. Semantic parity: compare bespoke and tree output after ANSI stripping and
   assert required tokens.
4. Positional parity: assert visible column positions for layout-sensitive
   output.

Width-dependent components must use a width matrix, initially `40`, `80`, and
`120` columns. Lists must additionally test ordered prefix transitions across
`9 -> 10`, `99 -> 100`, and non-default `start` values.

Every phase that adds native heading, list, or table rendering, or flips a
component to tree-backed terminal rendering, must re-run the darkmatter Flow A
parity gate (`render_tree_parity.rs`). A green component parity gate alone is
not enough to ship a renderer change.

## Inclusion With Differentiated Rigor

All seven proposed components should be included in group 1. They should not be
held to the same terminal parity bar at the same time.

`Section`, `UnorderedList`, and `OrderedList` are structural components. They
should reach terminal text and layout parity before their terminal renderers are
flipped.

`YamlBlock` should be included as a code-block migration. Full terminal parity
requires code-render hooks; without those hooks, only body and language parity
can be claimed.

`Progress` should be included as a semantic widget projection. It should
preserve label and value in all targets, while terminal glyph parity is gated on
render-hint handling.

`TwoColumn` should be included as a semantic projection first. Terminal-image
overlay behavior and terminal-app-specific cursor strategies remain bespoke in
group 1 unless a later phase explicitly takes them on.

`Table` should be included, but last. It forces table metadata, width planning,
two-pass rendering, and stricter positional parity.

## Test Strategy

### Shared Test Matrix

Use this matrix for every component:

| Tier | Purpose | Required for |
|------|---------|--------------|
| Structural snapshot | Catch projection shape regressions | all components |
| Validity | Ensure component trees satisfy tree validation | all components |
| Semantic parity | Ensure content survives after ANSI stripping | all components |
| Positional parity | Ensure layout-sensitive columns/indents match | sections, lists, columns, tables, progress layout |
| Strictness diagnostics | Ensure unsupported/lossy cases are explicit | all components with fallback paths |
| Darkmatter Flow A parity | Ensure native renderer changes do not regress parsed Markdown | heading, list, table renderer changes and component flips |

### Strictness Expectations

| Case | Strict | Warn | Lossy |
|------|--------|------|-------|
| unprojectable child component | render error or `Unsupported` escalated to error | fallback plus diagnostic | fallback, diagnostic optional |
| recursion depth overflow | error | `Unsupported` plus diagnostic | fallback if possible |
| accepted Prose styling loss | error if lossless output requested | plain text plus diagnostic | plain text |
| `TwoColumn` with `TerminalImage` in tree path | error | unsupported diagnostic and fallback/omission | fallback/omission |
| malformed render hint | error | ignore hint plus diagnostic | ignore hint |
| missing optional render hint | default behavior | default behavior | default behavior |

### Width Matrix

Use `Terminal::new_optimistic(width)` with deterministic widths.

- base matrix: `40`, `80`, `120`
- ordered-list prefix matrix: at least `1..=12`, `98..=102`, and a non-default
  `start`
- table matrix: at least one width that shows all columns and one width that
  drops optional columns
- two-column matrix: one stacked width and two side-by-side widths
- progress matrix: fixed bar widths plus at least one layout margin case

## Component Projection Contracts

### Section

Projection:

- `HeadingLevel::h1` through `h6` maps directly to `HeadingDepth(1)` through
  `HeadingDepth(6)`.
- `title` becomes inline heading children.
- each body item projects through `RenderableTerminalContent::to_tree_nodes()`.
- component layout becomes `renderable.layout.*` hints.
- no ANSI escapes appear in the tree.

Node shape:

```text
Section {
  depth,
  heading: [Text | inline nodes],
  children: [block nodes...],
}
```

Diagnostics:

- unsupported body component follows the shared strictness behavior
- styling flattened from `Prose` is recorded as accepted projection loss unless
  `Prose` has structural projection by then

Exit criteria:

- native `Heading` / `Section` terminal rendering exists before parity counts
- structural snapshot and validity tests pass
- semantic and positional parity pass for heading, body, margins, and multiple
  body items
- darkmatter Flow A parity remains green

### UnorderedList

Projection:

- projects to `NodeKind::List { ordered: false, start: None, children }`
- each item projects to `ListItem`
- `bullet`, `hanging_indent`, and `indent_children` become
  `renderable.list.*` hints
- existing `ListItem.checked` behavior is preserved for task-list items

Hint keys:

```text
renderable.list.bullet
renderable.list.hanging_indent
renderable.list.indent_children
```

Diagnostics:

- unsupported item component follows shared strictness behavior
- styling flattened from item content is diagnostic in `Warn`

Exit criteria:

- native list terminal rendering exists before parity counts
- custom bullets survive tree rendering
- disabled hanging indent survives tree rendering
- explicit and default `indent_children` behavior is tested
- nested block children are indented and not double-bulleted
- checked list item cases remain valid
- darkmatter Flow A parity remains green

### OrderedList

Projection:

- projects to `NodeKind::List { ordered: true, start, children }`
- each item projects to `ListItem`
- `indent_children` and hanging-indent behavior become list hints
- prefix width is not stored; the terminal renderer computes it from item index
  and `start`

Diagnostics:

- same unsupported and lossy behavior as `UnorderedList`

Exit criteria:

- native list terminal rendering exists before parity counts
- wrapping aligns for item `10` and item `100`
- non-default `start` values render correct prefixes and wrapping
- block-level children receive no number prefix and are indented
- nested lists respect reduced width
- darkmatter Flow A parity remains green

### YamlBlock

Projection:

- validated YAML body becomes `NodeKind::Code`
- `lang = Some("yaml")`
- code-block chrome and highlighter preferences become `renderable.code.*` hints
- component layout becomes layout hints

Hint keys:

```text
renderable.code.header_row = true
renderable.code.language_label = "yaml"
renderable.code.highlight = "preferred"
```

Diagnostics:

- missing code-render hook is not an error by itself; the default renderer emits
  plain code and records a diagnostic when strictness requires highlighted output
- malformed YAML never reaches projection because constructors validate it

Exit criteria:

- structural snapshot shows a single YAML code node
- empty YAML renders non-empty output with a YAML label
- body and language parity pass without hooks
- highlighting and chrome parity pass when darkmatter hooks are configured
- `TreeComponent` can carry code-render hooks into the terminal tree renderer

### Progress

Projection:

- visible fallback text is a paragraph or block span, e.g. `Upload 75%`
- `value` and `label` are represented in visible text and widget hints
- `bar_width`, fill/empty glyphs, and brackets are widget hints
- no `NodeKind::Progress` is added in group 1

Hint keys:

```text
renderable.widget.progress.value
renderable.widget.progress.bar_width
renderable.widget.progress.fill_char
renderable.widget.progress.empty_char
renderable.widget.progress.left_bracket
renderable.widget.progress.right_bracket
```

Diagnostics:

- missing terminal glyph hints use defaults
- malformed value hint is an error in `Strict`, diagnostic and fallback text in
  `Warn`

Exit criteria:

- value is clamped before projection just as bespoke rendering does
- Markdown output contains label and percentage fallback
- browser output contains label and percentage fallback or semantic progress
  markup
- terminal tree rendering honors custom glyphs when hints are present
- layout margin parity passes for simple cases

### TwoColumn

Projection:

- left and right content each project through the shared content projection
  layer
- projection preserves left/right order and stores gap and width preferences in
  widget hints
- no `NodeKind::Columns` is added in group 1
- terminal image columns are explicitly unsupported in the tree path for group 1

Hint keys:

```text
renderable.widget.columns.gap
renderable.widget.columns.left_width.kind
renderable.widget.columns.left_width.value
renderable.widget.columns.stack_below
```

Diagnostics:

- `TerminalImage` in either column produces an unsupported diagnostic in `Warn`
  and an error in `Strict`
- unsupported child component follows shared strictness behavior

Exit criteria:

- structural snapshot preserves two child regions
- Markdown fallback preserves left/right content, preferably as a two-column
  table
- browser fallback preserves left/right content with CSS-ready classes or data
  attributes
- terminal text/prose-only parity passes for stacked and side-by-side widths
- full image overlay parity remains out of scope for group 1

### Table

Projection:

- projects to `NodeKind::Table`, `TableRow`, and `TableCell`
- column metadata is stored in `renderable.table.column.*` hints
- each cell gets readable pre-formatted text
- original typed cell data and alignment are stored in
  `renderable.table.cell.*` hints
- layout, striping, cursor preference, and drop behavior are stored as hints

Hint keys:

```text
renderable.table.column.{i}.min_width
renderable.table.column.{i}.max_width
renderable.table.column.{i}.fixed_width
renderable.table.column.{i}.conditional
renderable.table.column.{i}.drop_note
renderable.table.column.{i}.uniform_alignment
renderable.table.cell.kind
renderable.table.cell.raw_value
renderable.table.cell.alignment
renderable.table.cell.vertical_alignment
renderable.terminal.prefer_cursor_alignment
renderable.terminal.alternate_background
renderable.terminal.alternate_text_color
```

Diagnostics:

- malformed table hints are errors in `Strict` and diagnostics in `Warn`
- styling loss from pre-rendered `Prose` cells is diagnostic in `Warn`

Exit criteria:

- terminal tree renderer uses a native two-pass table handler
- width planning works across the table width matrix
- conditional columns and drop notes match bespoke behavior
- typed numeric and currency cells are readable and right-aligned
- multi-line row height and vertical alignment match bespoke behavior
- striping survives SGR resets inside cells
- cursor preference is honored in terminal and ignored by Markdown/browser
- darkmatter Flow A parity remains green

## Implementation Sequence

### Phase -1: Decisions Gate

This phase is documentation and test-design work, not broad code migration.

Deliver:

- optional `TerminalRenderable` tree method decision recorded in the API plan
- `NodeKind::Section` decision recorded with renderer/serde/test impact
- no new `Progress` / `Columns` node decision recorded
- hint namespace and typed helper API sketch
- browser adapter error policy
- code-render hook wiring plan through `TreeComponent`

Exit criteria:

- the decisions in this spec are accepted or explicitly amended before Phase 0
  implementation begins

### Phase 0: Shared Foundations

Deliver:

- optional tree method on `TerminalRenderable`
- projection context with recursion depth guard
- `RenderableTerminalContent::to_tree_nodes()` or equivalent
- projected nodes use `SourceSpan::synthetic()`; projection diagnostics use the
  shared `renderable::tree::Diagnostic` type
- typed hint helpers over `NodeAttrs.data`, generic over a namespace root
- `TreeRenderable::tree_layout_hints()` default method
- `TreeComponent` layout initialization from wrapped component hints
- extensions to existing `TerminalRenderContext`
- browser tree adapter with `Warn` default policy
- shared Level 1 parity helper module in `biscuit-terminal/lib/tests/`

Synthetic test fixtures:

- `StubTreeComponent` returns `Some(RenderNode)` through the optional method
- `StubBespokeOnly` returns `None`
- `StubRecursiveComponent` exceeds projection depth

Exit criteria:

- string projection, tree-capable projection, bespoke-only fallback, strict
  unsupported behavior, warning diagnostics, and recursion overflow are all
  tested
- browser adapter renders a visible fallback for a tree render error
- no production component renderer is flipped in this phase

### Phase 1: Section

Deliver:

- `NodeKind::Section` model, serde, validation, Markdown renderer, browser
  renderer, and native terminal renderer
- `Section::render_tree()`
- section structural snapshots, validity tests, semantic parity, positional
  parity, strictness tests, and darkmatter Flow A parity

What we learn:

- whether `Section` is the right first new node variant
- whether the content projection layer handles multiple child items
- whether component layout hints survive tree rendering

Exit criteria:

- terminal `Heading` and `Section` rendering no longer delegates to
  `Section::render()` for the parity path
- multiple body items remain separate
- margins and heading positions match bespoke output for Level 1 tests
- parsed Markdown heading parity remains green

### Phase 2: UnorderedList and OrderedList

Deliver:

- list hint helpers
- native terminal list renderer
- `UnorderedList::render_tree()`
- `OrderedList::render_tree()`
- list structural snapshots, validity tests, semantic parity, positional
  parity, strictness tests, width matrix tests, and darkmatter Flow A parity

What we learn:

- whether hints are enough for custom bullets and hanging indent
- whether width and indentation compound correctly through nested list trees
- whether list item structure is sufficient to infer inline, block, and mixed
  rendering behavior

Exit criteria:

- terminal `List` rendering no longer delegates to list components for the
  parity path
- unordered custom bullets and disabled hanging indent survive projection
- ordered prefix width works across `9 -> 10`, `99 -> 100`, and custom `start`
- checked list items remain valid and covered
- parsed Markdown list parity remains green

### Phase 3: YamlBlock

Deliver:

- `YamlBlock::render_tree()`
- code-block hint helpers
- terminal and browser code-render hook support
- `TreeComponent` hook wiring
- YAML structural snapshots, validity tests, body parity, empty-YAML tests, and
  highlighting/chrome parity with darkmatter hooks enabled

What we learn:

- whether renderer hooks preserve rich code-block behavior without moving
  darkmatter dependencies into `renderable`
- whether adapter-level hook wiring is sufficient for component rendering

Exit criteria:

- plain tree rendering preserves YAML body and language label
- darkmatter hook rendering preserves syntax highlighting and code-block chrome
- empty YAML produces visible output
- existing `YamlBlock` bespoke parity with Markdown fences remains green

### Phase 4: Progress

Deliver:

- `Progress::render_tree()`
- progress widget hint helpers
- terminal renderer recognition of progress hints
- Markdown/browser fallback behavior
- structural snapshots, validity tests, semantic parity, glyph tests, layout
  tests, and strictness tests

What we learn:

- whether widget hints are enough before adding a generic widget node
- how non-terminal renderers should degrade visual components

Exit criteria:

- label and percentage survive in all targets
- terminal tree rendering honors `bar_width` and custom glyphs from hints
- malformed progress hints follow strictness expectations
- bespoke terminal rendering remains primary until parity is judged sufficient

### Phase 5: TwoColumn

Deliver:

- `TwoColumn::render_tree()`
- columns widget hint helpers
- text/prose-only terminal tree rendering for stacked and side-by-side layouts
- Markdown/browser semantic fallback
- strict unsupported diagnostics for terminal-image columns
- structural snapshots, validity tests, semantic parity, positional parity,
  width matrix tests, and strictness tests

What we learn:

- whether widget hints can represent layout primitives without new `NodeKind`
  variants
- whether forked render contexts can constrain child widths correctly
- where the boundary should remain between tree-rendered layout and bespoke
  terminal image overlay behavior

Exit criteria:

- left/right content survives in all targets
- side-by-side gap and stacked fallback are positionally tested
- terminal-image columns produce explicit diagnostics in the tree path
- full image overlay parity remains out of scope

### Phase 6: Table

Deliver:

- `Table::render_tree()`
- table column and cell hint helpers
- native two-pass terminal table renderer
- table structural snapshots, validity tests, semantic parity, positional
  parity, width matrix tests, strictness tests, and darkmatter Flow A parity

What we learn:

- whether table metadata can live in hints without bloating `NodeKind::Table`
- whether two-pass node rendering works cleanly inside the exhaustive match
  architecture
- whether lossy projection of styled cells is acceptable or `Prose` needs a
  structural projection before flipping table rendering

Exit criteria:

- terminal `Table` rendering no longer delegates to `Table::render()` for the
  parity path
- width planning, conditional columns, drop notes, typed cells, row heights,
  vertical alignment, cursor preference, striping, layout margins, and row fill
  are covered
- parsed Markdown table parity remains green

## Milestones

### Milestone A: Projection Infrastructure

Includes Phase -1 and Phase 0.

Exit criteria:

- projection mechanism is settled and implemented
- browser adapter and render option plumbing exist
- shared test helpers exist
- no production renderer is flipped

### Milestone B: Structural Blocks

Includes Phase 1 and Phase 2.

Exit criteria:

- `Section`, `UnorderedList`, and `OrderedList` have valid tree projections
- native section and list terminal renderers replace component delegation in the
  parity path
- component parity and darkmatter Flow A parity are green

### Milestone C: Code Blocks and Widgets

Includes Phase 3 and Phase 4.

Exit criteria:

- `YamlBlock` and `Progress` have semantic projections
- code hooks and progress hints preserve rich terminal behavior where configured
- Markdown and browser degradation remains meaningful

### Milestone D: Layout Primitives

Includes Phase 5.

Exit criteria:

- `TwoColumn` has a semantic projection
- text/prose terminal tree rendering works at multiple widths
- image overlay behavior remains explicitly bespoke or unsupported in tree mode

### Milestone E: Tables

Includes Phase 6.

Exit criteria:

- `Table` has a valid projection with metadata hints
- native two-pass table rendering works
- table component parity and darkmatter Flow A parity are green

## Remaining Open Questions

These questions do not block Phase 0, but they should be revisited before
flipping broad production rendering paths:

- Should a later phase add a generic `NodeKind::Widget` once several widget-like
  components have proven the hint-based approach?
- Should `Prose` get a full structural projection before `Table` is flipped, or
  is documented styling-loss diagnostics enough for group 1?
- Should table column metadata eventually become typed fields on
  `NodeKind::Table` if the hint payload grows too large?
- What exact strictness behavior should browser adapters expose beyond their
  default `Warn` policy?
- Should `Provenance` gain a first-class component/origin variant so a spliced
  component subtree can name its originating component, instead of being
  indistinguishable from other `Synthetic` nodes? `SourceDescriptor::Component`
  already exists, but no `Provenance` variant references it outside
  `Transcluded`.

## Recommended Initial Scope

Start with Phase -1 and Phase 0 exactly as written. The most important work is
not `Section` itself; it is the projection mechanism for
`Rc<dyn TerminalRenderable>`, the typed hint API, layout transfer through the
existing `TreeComponent`, extensions to the existing terminal context, the
browser adapter, and Level 1 parity infrastructure.

Once those are in place, implement `Section`, `UnorderedList`, and
`OrderedList`. These three are structural, widely useful, and force the tree
renderer to solve real component fidelity before taking on code highlighting,
widgets, column layout, or table width planning.

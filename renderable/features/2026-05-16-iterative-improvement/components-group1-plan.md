---
last_updated: "2026-05-16"
source_spec: renderable/features/2026-05-16-iterative-improvement/components-group1-spec.md
confidence: high
---

# Group 1 Implementation Plan

High-confidence plan derived from the Group 1 Tree Rendering Component
Specification. Each phase lists concrete code changes, file locations, test
files, and verification steps.

## Codebase Starting Point

### Crate Inventory

| Crate | Location | Role in this plan |
|-------|----------|-------------------|
| `renderable` | `renderable/` | `NodeKind`, `RenderNode`, `NodeAttrs`, `SourceSpan`, `Diagnostic`, `TreeRenderable`, tree renderers (markdown/browser), validation |
| `biscuit-terminal` (lib) | `biscuit-terminal/lib/` | `TerminalRenderable`, `RenderableTerminalContent`, `TreeComponent`, terminal tree renderer, all seven components |
| `darkmatter` (lib) | `darkmatter/lib/` | `YamlBlock`, code-block helpers, darkmatter Flow A parity tests |

### Key Types Today

- `NodeKind` — 22 variants, exhaustive match in 3 renderers + validation
  (`renderable/src/tree/node.rs:82`)
- `NodeAttrs` — `{ id, classes, data: BTreeMap<String, Value> }`
  (`renderable/src/tree/attrs.rs:26`)
- `SourceSpan` — `{ provenance, location }`, `Provenance::Synthetic` already
  exists (`renderable/src/tree/source.rs:139`)
- `Diagnostic` — `{ kind, severity, message, span }`, kinds include
  `Unsupported` and `Lossy` (`renderable/src/tree/diagnostic.rs:38`)
- `TreeRenderable` — single method `render_tree(&self) -> RenderNode`
  (`renderable/src/tree/mod.rs:54`)
- `TerminalRenderable` — `render`, `render_optimistic`, `layout`, `as_any`
  (`biscuit-terminal/lib/src/components/renderable.rs:17`)
- `RenderableTerminalContent` — `String(String)` |
  `Component(Rc<dyn TerminalRenderable>)`
  (`biscuit-terminal/lib/src/components/renderable.rs:247`)
- `TreeComponent<T>` — adapter wrapping `TreeRenderable` into
  `TerminalRenderable` (`biscuit-terminal/lib/src/render_tree/component.rs:51`)
- `TerminalRenderContext` — `width`, `color_depth`, `hyperlinks`,
  `image_support`, `layout`, `terminal`
  (`biscuit-terminal/lib/src/render_tree/options.rs:34`)

### Current Delegation Points

The terminal tree renderer delegates to bespoke components:

- `NodeKind::Heading` → `Section::render()` (`render.rs:154`)
- `NodeKind::List` → `OrderedList`/`UnorderedList::render()` (`render.rs:351-354`)
- `NodeKind::Table` → `Table::render()` (`render.rs:419-420`)

These are the delegation loops that must be broken per Architecture
Enhancement 5.

---

## Phase -1: Decisions Gate

**Scope**: Documentation and test-design only. No production code changes.

### Deliverables

1. Record that the spec's settled decisions are accepted (this plan is that
   record).
2. Confirm `NodeKind::Section` is the only new node variant for group 1.
3. Confirm no `NodeKind::Progress` or `NodeKind::Columns`.
4. Confirm hint namespace pattern: `renderable.{layout,list,table,widget.*,
   code,terminal}.*` over `NodeAttrs.data`.
5. Confirm browser adapter default: `RenderStrictness::Warn`.
6. Confirm code-render hook wiring plan through `TreeComponent`.
7. Confirm projected nodes use `SourceSpan::synthetic()`.
8. Confirm projection diagnostics reuse `renderable::tree::Diagnostic`.

### Exit Criteria

- [ ] This plan is accepted as the decisions record
- [ ] No open questions block Phase 0

---

## Phase 0: Shared Foundations

**Scope**: Infrastructure in `renderable` and `biscuit-terminal`. No component
is flipped to tree-backed rendering.

### 0.1 Add `NodeKind::Section`

**Files changed**:

| File | Change |
|------|--------|
| `renderable/src/tree/node.rs` | Add `Section { depth, heading, children }` variant after `Heading` |
| `renderable/src/tree/node.rs` | Add `RenderNode::section()` builder |
| `renderable/src/tree/node.rs` | Update `children()` and `children_mut()` match arms |
| `renderable/src/tree/validate.rs` | Update `is_block()`, `is_phrasing_only()`, `kind_name()`, containment rules |
| `renderable/src/tree/validate.rs` | `Section` children must be block-level; heading children must be phrasing |
| `renderable/src/tree/render/markdown.rs` | Add `NodeKind::Section` arm: emit heading then blank line then body |
| `renderable/src/tree/render/browser.rs` | Add `NodeKind::Section` arm: `<section>` with heading tag + children |
| `renderable/src/tree/render/browser.rs` | Update exhaustive match in `Writer::render()` |

**Node shape**:

```rust
NodeKind::Section {
    depth: HeadingDepth,
    heading: Vec<RenderNode>,
    children: Vec<RenderNode>,
}
```

**Validation rules**:

- `Section` is block-level
- `Section.heading` children must be phrasing content
- `Section.children` must be block-level
- `Section` may appear inside `Root`, `BlockQuote`, `Section`, `ListItem`

**Tests**:

- `renderable/src/tree/node.rs` — builder round-trip, children access
- `renderable/src/tree/validate.rs` — block-in-heading error, valid section
- `renderable/src/tree/render/markdown.rs` — section renders as heading +
  body
- `renderable/src/tree/render/browser.rs` — section renders as `<section>`

### 0.2 Typed Hint Helpers

**Files changed**:

| File | Change |
|------|--------|
| `renderable/src/tree/attrs.rs` | Add `HintNamespace` newtype and typed read/write helpers |

**API sketch**:

```rust
pub struct HintNamespace(&'static str);

impl HintNamespace {
    pub const LAYOUT: HintNamespace = HintNamespace("renderable.layout");
    pub const LIST: HintNamespace = HintNamespace("renderable.list");
    pub const TABLE: HintNamespace = HintNamespace("renderable.table");
    pub const CODE: HintNamespace = HintNamespace("renderable.code");
    pub const TERMINAL: HintNamespace = HintNamespace("renderable.terminal");
    pub const WIDGET_PROGRESS: HintNamespace = HintNamespace("renderable.widget.progress");
    pub const WIDGET_COLUMNS: HintNamespace = HintNamespace("renderable.widget.columns");
}

impl NodeAttrs {
    pub fn set_hint(&mut self, ns: HintNamespace, key: &str, value: Value);
    pub fn get_hint(&self, ns: HintNamespace, key: &str) -> Option<&Value>;
    pub fn remove_hint(&mut self, ns: HintNamespace, key: &str);
}
```

The helper layer does **not** hard-code the `renderable` prefix. `HintNamespace`
is a transparent wrapper. Other crates can define their own namespace roots.

**Tests**:

- Round-trip set/get/remove
- Namespaced keys do not collide with ad-hoc keys
- JSON serialization preserves namespaced keys

### 0.3 Optional Tree Method on `TerminalRenderable`

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/components/renderable.rs` | Add default `fn render_tree_node(&self) -> Option<RenderNode> { None }` to `TerminalRenderable` |

**Notes**:

- Method name `render_tree_node` distinguishes it from
  `TreeRenderable::render_tree()`.
- Default returns `None` so all existing impls are compatible.
- No downcasting needed.

**Tests**:

- Stub type returning `None` compiles and returns `None`
- Stub type returning `Some(node)` compiles and returns node

### 0.4 Tree Content Projection Layer

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/render_tree/mod.rs` | Add `pub mod projection;` |
| `biscuit-terminal/lib/src/render_tree/projection.rs` (new) | `TreeProjectionContext`, `ProjectionResult`, `to_tree_nodes` on `RenderableTerminalContent` |

**API sketch**:

```rust
pub struct TreeProjectionContext {
    pub strictness: RenderStrictness,
    pub max_depth: usize,
    pub current_depth: usize,
}

pub struct ProjectionResult {
    pub nodes: Vec<RenderNode>,
    pub diagnostics: Vec<Diagnostic>,
}
```

**Behavior**:

- `RenderableTerminalContent::String(s)` → `Text` or `Paragraph` node
- `RenderableTerminalContent::Component(c)` → calls
  `c.render_tree_node()`
- `Some(node)` → inserted into parent
- `None` → `Unsupported` in `Strict`, ANSI-stripped fallback + diagnostic in
  `Warn`/`Lossy`
- Recursion depth guard: overflow → `DiagnosticKind::Unsupported` diagnostic
  + `Unsupported` node or error
- Diagnostics use existing `renderable::tree::Diagnostic` type
- Projected nodes use `SourceSpan::synthetic()`

**Tests**:

- `StubTreeComponent` → `Some(RenderNode::text("hello"))`
- `StubBespokeOnly` → `None`
- `StubRecursiveComponent` → exceeds depth, produces diagnostic
- String content → Text node
- Strict unsupported → error
- Warn unsupported → fallback + diagnostic

### 0.5 `TreeRenderable::tree_layout_hints()` Default Method

**Files changed**:

| File | Change |
|------|--------|
| `renderable/src/tree/mod.rs` | Add `fn tree_layout_hints(&self) -> Option<LayoutHints> { None }` to `TreeRenderable` |

**Notes**:

- Default returns `None` so existing impls are compatible.
- `LayoutHints` struct defined alongside (or imported from attrs).

### 0.6 Extend `TerminalRenderContext`

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/render_tree/options.rs` | Add fields and fork helpers |

**Additions**:

```rust
pub struct TerminalRenderContext {
    // existing fields...
    pub available_width: u32,
    pub current_indent: u32,
    pub active_layout_hints: Option<LayoutHints>,
}

impl TerminalRenderContext {
    pub fn for_child(&self, indent_delta: u32, width_delta: u32) -> Self;
    pub fn with_width(&self, available_width: u32) -> Self;
    pub fn with_layout(&self, layout: LayoutHints) -> Self;
}
```

**Notes**:

- `width` retains its current meaning (root terminal width).
- `available_width` tracks the current renderable width after indentation and
  column constraints.
- Initialize `available_width` to `width` in `from_terminal()` and `fallback()`.

### 0.7 Browser Tree Adapter

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/render_tree/mod.rs` | Add `pub mod browser_adapter;` |
| `biscuit-terminal/lib/src/render_tree/browser_adapter.rs` (new) | `BrowserTreeComponent<T>` |

**Behavior**:

- Wraps `BrowserRenderable` similarly to how `TreeComponent` wraps
  `TreeRenderable`.
- Default `RenderStrictness::Warn`: structural errors → visible unsupported
  fragment + diagnostic; non-fatal losses → diagnostics.
- Callers needing strict behavior use the lower-level `render_browser_node`
  directly.

**Tests**:

- Valid component → renders HTML
- Invalid tree → visible fallback, no panic
- Warn diagnostics collected

### 0.8 Shared Parity Test Helpers

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/tests/parity_helpers.rs` (new) | `Terminal::new_optimistic(width)`, ANSI stripping, width matrix, normalization |

**Notes**:

- Located next to existing component parity tests.
- All group 1 parity tests import from this module.

### 0.9 Synthetic Test Fixtures

Add to `biscuit-terminal/lib/src/render_tree/projection.rs` tests:

| Fixture | Behavior |
|---------|----------|
| `StubTreeComponent` | Returns `Some(RenderNode::text("stub"))` from `render_tree_node()` |
| `StubBespokeOnly` | Returns `None` from `render_tree_node()` |
| `StubRecursiveComponent` | Returns a node containing itself, exceeding projection depth |

### Phase 0 Exit Criteria

- [ ] `NodeKind::Section` compiles, serializes, validates, renders in markdown
  and browser
- [ ] Typed hint helpers read/write namespaced keys without collision
- [ ] `TerminalRenderable::render_tree_node()` has default `None`, existing
  impls compile
- [ ] `RenderableTerminalContent::to_tree_nodes()` projects strings and
  components
- [ ] Recursion depth guard produces diagnostics on overflow
- [ ] `TreeRenderable::tree_layout_hints()` has default `None`
- [ ] `TerminalRenderContext` has `available_width`, `current_indent`, fork
  helpers
- [ ] Browser tree adapter renders valid HTML and degrades on error
- [ ] No production component renderer is flipped

---

## Phase 1: Section

### 1.1 `Section::render_tree_node()`

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/components/section.rs` | Implement `render_tree_node()` returning `Option<RenderNode>` |

**Projection logic**:

- `HeadingLevel::h1..h6` → `HeadingDepth(1..6)`
- `title` → inline heading children (via `to_tree_nodes` on title string)
- Each body item → `to_tree_nodes()` through projection layer
- Layout → `renderable.layout.*` hints
- Return `Some(NodeKind::Section { depth, heading, children })`

### 1.2 Native Heading/Section Terminal Rendering

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/render_tree/render.rs` | Replace `NodeKind::Heading` delegation to `Section::render()` with native rendering |
| `biscuit-terminal/lib/src/render_tree/render.rs` | Add `NodeKind::Section` arm with native rendering |

**Native heading rendering**:

- Compute prefix from depth (`#`, `##`, etc.)
- Apply heading style (bold h1-h3, italic h4-h5, plain h6)
- Render inline children via `render_inline()`
- Apply layout hints from `active_layout_hints`

**Native section rendering**:

- Render heading part as above
- Render body children via `render_blocks()`
- Apply layout hints (margins from `renderable.layout.*`)

### 1.3 Section Tests

**Test file**: `biscuit-terminal/lib/tests/section_parity.rs` (new)

| Tier | Test |
|------|------|
| Structural snapshot | `render_tree()` → JSON snapshot via insta |
| Validity | `validate()` → zero error-severity findings |
| Semantic parity | ANSI-stripped bespoke vs tree output contains same tokens |
| Positional parity | Heading position, body margin, multiple body items |
| Strictness | Unsupported child component produces diagnostic |
| Darkmatter Flow A | Re-run `render_tree_parity.rs` — must remain green |

**Width matrix**: 40, 80, 120

### Phase 1 Exit Criteria

- [ ] Terminal heading/section rendering no longer delegates to
  `Section::render()`
- [ ] Multiple body items remain separate
- [ ] Margins and heading positions match bespoke output
- [ ] Parsed Markdown heading parity remains green
- [ ] Structural snapshot, validity, semantic parity, positional parity all pass

---

## Phase 2: UnorderedList and OrderedList

### 2.1 List Hint Helpers

**Files changed**:

| File | Change |
|------|--------|
| `renderable/src/tree/attrs.rs` | Add `ListRenderHints` struct with `bullet`, `hanging_indent`, `indent_children` fields |
| `renderable/src/tree/attrs.rs` | Add typed read/write via `HintNamespace::LIST` |

### 2.2 `UnorderedList::render_tree_node()` and `OrderedList::render_tree_node()`

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/components/list.rs` | Implement `render_tree_node()` for `UnorderedList` |
| `biscuit-terminal/lib/src/components/list.rs` | Implement `render_tree_node()` for `OrderedList` |

**Projection logic**:

- Unordered → `NodeKind::List { ordered: false, start: None, children }`
- Ordered → `NodeKind::List { ordered: true, start: None, children }`
  (prefix width computed by renderer from index)
- Each item → `NodeKind::ListItem { checked, children }` where children come
  from `to_tree_nodes()`
- `bullet`, `hanging_indent`, `indent_children` → `renderable.list.*` hints
- Preserve `checked` field for task-list items

### 2.3 Native List Terminal Rendering

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/render_tree/render.rs` | Replace list delegation with native rendering |

**Native rendering rules**:

- Ordered prefix width computed from index + `start` (e.g., `9.` → 3 chars,
  `10.` → 4 chars, `100.` → 5 chars)
- Unordered bullet width from hint or default `•`
- Inline-only items → prefix + text
- Block-only items → indented, no prefix
- Mixed items → first paragraph with prefix, following blocks indented
- Checked items → `[x]` / `[ ]` prefix
- Hanging indent from hint (default enabled)
- `indent_children` from hint for nested content

### 2.4 List Tests

**Test file**: `biscuit-terminal/lib/tests/list_parity.rs` (new)

| Tier | Test |
|------|------|
| Structural snapshot | Both list types → JSON snapshots |
| Validity | `validate()` → zero error-severity findings |
| Semantic parity | ANSI-stripped bespoke vs tree output |
| Positional parity | Indentation, prefix alignment, wrapping |
| Strictness | Unsupported child, depth overflow |
| Darkmatter Flow A | Re-run `render_tree_parity.rs` |

**Width matrix**:

- Base: 40, 80, 120
- Ordered prefix transitions: 1..=12, 98..=102
- Non-default `start`: start=5, start=99

**Custom bullet tests**: bullet="*", bullet="→"

**Checked item tests**: checked=true, checked=false, mixed

**Nested block children**: paragraph + code block in same item

### Phase 2 Exit Criteria

- [ ] Terminal list rendering no longer delegates to list components
- [ ] Custom bullets survive tree rendering
- [ ] Disabled hanging indent survives tree rendering
- [ ] `indent_children` behavior tested with explicit and default values
- [ ] Nested block children indented, not double-bulleted
- [ ] Checked items remain valid
- [ ] Ordered prefix width correct at 9→10, 99→100, custom start
- [ ] Parsed Markdown list parity remains green

---

## Phase 3: YamlBlock

### 3.1 Code-Block Hint Helpers

**Files changed**:

| File | Change |
|------|--------|
| `renderable/src/tree/attrs.rs` | Add `CodeRenderHints` with `header_row`, `language_label`, `highlight` |

### 3.2 Code-Render Hook Trait

**Files changed**:

| File | Change |
|------|--------|
| `renderable/src/tree/render/mod.rs` | Define `CodeRenderer` trait with `render_terminal_code` and `render_browser_code` |
| `renderable/src/tree/render/mod.rs` | Default impl returns `None` (plain behavior) |

**API sketch**:

```rust
pub trait CodeRenderer {
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

### 3.3 Hook Wiring Through `TreeComponent` and Render Options

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/render_tree/component.rs` | Add `code_renderer: Option<Rc<dyn CodeRenderer>>` to `TreeComponent` |
| `biscuit-terminal/lib/src/render_tree/options.rs` | Add `code_renderer: Option<Rc<dyn CodeRenderer>>` to `TerminalRenderOptions` |
| `biscuit-terminal/lib/src/render_tree/render.rs` | Check `opts.code_renderer` in `NodeKind::Code` arm; fall back to plain rendering |

### 3.4 `YamlBlock::render_tree_node()`

**Files changed**:

| File | Change |
|------|--------|
| `darkmatter/lib/src/markdown/yaml_block.rs` | Implement `render_tree_node()` |

**Projection logic**:

- Body → `NodeKind::Code { lang: Some("yaml"), meta: None, value }`
- Code hints → `renderable.code.header_row`, `language_label`, `highlight`
- Layout → `renderable.layout.*` hints

### 3.5 YamlBlock Tests

**Test file**: `darkmatter/lib/tests/yaml_block_parity.rs` (new)

| Tier | Test |
|------|------|
| Structural snapshot | Single YAML code node |
| Validity | Zero error-severity findings |
| Body parity | Plain tree rendering preserves YAML body + "yaml" label |
| Empty YAML | Non-empty output with YAML label |
| Highlighting parity | Darkmatter hook → syntax highlighting + chrome preserved |
| Markdown output | YAML fences preserved |
| Bespoke parity | Existing `YamlBlock` bespoke tests remain green |

### Phase 3 Exit Criteria

- [ ] Plain tree rendering preserves YAML body and language label
- [ ] Darkmatter hook rendering preserves syntax highlighting and chrome
- [ ] Empty YAML produces visible output
- [ ] `TreeComponent` carries code-render hooks into terminal renderer
- [ ] Existing `YamlBlock` bespoke parity with Markdown fences remains green

---

## Phase 4: Progress

### 4.1 Progress Hint Helpers

**Files changed**:

| File | Change |
|------|--------|
| `renderable/src/tree/attrs.rs` | Add `ProgressHints` struct |

### 4.2 `Progress::render_tree_node()`

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/components/progress.rs` | Implement `render_tree_node()` |

**Projection logic**:

- Visible text: `Paragraph` with `"{label} {percentage}%"` (e.g., "Upload 75%")
- Widget hints: `renderable.widget.progress.{value, bar_width, fill_char,
  empty_char, left_bracket, right_bracket}`
- No `NodeKind::Progress` — uses `Paragraph` with hints
- Value is clamped before projection (same as bespoke)

### 4.3 Terminal Renderer Recognition of Progress Hints

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/render_tree/render.rs` | In `Paragraph` arm, check for `renderable.widget.progress.*` hints; if present, render progress bar |

### 4.4 Markdown/Browser Fallback

**Files changed**:

| File | Change |
|------|--------|
| `renderable/src/tree/render/markdown.rs` | `Paragraph` with progress hints → label + percentage text |
| `renderable/src/tree/render/browser.rs` | `Paragraph` with progress hints → semantic `<progress>` or label fallback |

### 4.5 Progress Tests

**Test file**: `biscuit-terminal/lib/tests/progress_parity.rs` (new)

| Tier | Test |
|------|------|
| Structural snapshot | Paragraph with progress hints |
| Validity | Zero error-severity findings |
| Semantic parity | Label and percentage survive in all targets |
| Glyph tests | Custom fill/empty/bracket chars honored |
| Layout tests | Bar width + margin |
| Strictness | Malformed value hint: error in Strict, diagnostic in Warn |

**Progress matrix**: fixed bar widths, layout margin case

### Phase 4 Exit Criteria

- [ ] Label and percentage survive in all targets (terminal, markdown, browser)
- [ ] Terminal tree rendering honors `bar_width` and custom glyphs from hints
- [ ] Malformed progress hints follow strictness expectations
- [ ] Bespoke terminal rendering remains primary until parity is judged
  sufficient

---

## Phase 5: TwoColumn

### 5.1 Columns Hint Helpers

**Files changed**:

| File | Change |
|------|--------|
| `renderable/src/tree/attrs.rs` | Add `ColumnsHints` struct |

### 5.2 `TwoColumn::render_tree_node()`

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/components/two_column.rs` | Implement `render_tree_node()` |

**Projection logic**:

- Left and right content each projected through `to_tree_nodes()`
- Store in a block container preserving order
- Hints: `renderable.widget.columns.{gap, left_width.kind,
  left_width.value, stack_below}`
- `TerminalImage` in either column → `Unsupported` diagnostic (Warn) or error
  (Strict)

### 5.3 Terminal Tree Rendering for TwoColumn

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/render_tree/render.rs` | Detect columns hints on container; render side-by-side or stacked |

**Rendering rules**:

- Text/prose-only children → side-by-side or stacked based on width + hints
- Fork context for child widths: `for_child(0, gap/2)` for each column
- Image overlay behavior remains bespoke/explicitly unsupported

### 5.4 Markdown/Browser Fallback

- Markdown → two-column table or sequential sections
- Browser → flex container with CSS-ready classes/data attributes

### 5.5 TwoColumn Tests

**Test file**: `biscuit-terminal/lib/tests/two_column_parity.rs` (new)

| Tier | Test |
|------|------|
| Structural snapshot | Two child regions preserved |
| Validity | Zero error-severity findings |
| Semantic parity | Left/right content survives in all targets |
| Positional parity | Side-by-side gap, stacked fallback |
| Strictness | TerminalImage → diagnostic (Warn), error (Strict) |

**Width matrix**: one stacked width, two side-by-side widths

### Phase 5 Exit Criteria

- [ ] Left/right content survives in all targets
- [ ] Side-by-side gap and stacked fallback positionally tested
- [ ] Terminal-image columns produce explicit diagnostics
- [ ] Full image overlay parity remains out of scope

---

## Phase 6: Table

### 6.1 Table Hint Helpers

**Files changed**:

| File | Change |
|------|--------|
| `renderable/src/tree/attrs.rs` | Add table column and cell hint types |

**Hint keys**:

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

### 6.2 `Table::render_tree_node()`

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/components/table/table.rs` | Implement `render_tree_node()` |

**Projection logic**:

- Projects to `NodeKind::Table`, `TableRow`, `TableCell`
- Each cell → readable pre-formatted `Text` node
- Original typed data → `renderable.table.cell.*` hints
- Alignment → `renderable.table.cell.alignment` hint
- Column metadata → `renderable.table.column.{i}.*` hints
- Layout, striping, cursor preference → terminal hints

### 6.3 Native Two-Pass Table Terminal Rendering

**Files changed**:

| File | Change |
|------|--------|
| `biscuit-terminal/lib/src/render_tree/render.rs` | Replace `NodeKind::Table` delegation with native two-pass renderer |

**Two-pass approach**:

1. **Pre-scan pass**: iterate rows/cells, resolve column widths, row heights,
   border widths, drop state, striping
2. **Emit pass**: render cells with computed widths, alignment, striping

**Features covered**:

- Width planning across table width matrix
- Conditional columns and drop notes
- Typed numeric/currency cells → readable + right-aligned
- Multi-line row height + vertical alignment
- Striping surviving SGR resets
- Cursor preference honored in terminal, ignored by markdown/browser

### 6.4 Table Tests

**Test file**: `biscuit-terminal/lib/tests/table_parity.rs` (new)

| Tier | Test |
|------|------|
| Structural snapshot | Table with metadata hints |
| Validity | Zero error-severity findings |
| Semantic parity | All cell content survives |
| Positional parity | Column alignment, row heights, borders |
| Width matrix | Full-width (all columns) + narrow (drops conditional columns) |
| Strictness | Malformed hints, styling loss from Prose cells |
| Darkmatter Flow A | Re-run `render_tree_parity.rs` |

**Table width matrix**: at least one width showing all columns, one width
dropping optional columns

### Phase 6 Exit Criteria

- [ ] Terminal `Table` rendering no longer delegates to `Table::render()`
- [ ] Width planning works across table width matrix
- [ ] Conditional columns and drop notes match bespoke behavior
- [ ] Typed numeric and currency cells are readable and right-aligned
- [ ] Multi-line row height and vertical alignment match bespoke behavior
- [ ] Striping survives SGR resets inside cells
- [ ] Cursor preference honored in terminal, ignored by markdown/browser
- [ ] Parsed Markdown table parity remains green

---

## Milestones Summary

| Milestone | Phases | Key Gate |
|-----------|--------|----------|
| A: Projection Infrastructure | -1, 0 | Projection mechanism, browser adapter, test helpers — no production flip |
| B: Structural Blocks | 1, 2 | Section + both lists have valid tree projections and native terminal renderers |
| C: Code Blocks and Widgets | 3, 4 | YamlBlock + Progress have semantic projections; code hooks preserve highlighting |
| D: Layout Primitives | 5 | TwoColumn has semantic projection; text/prose terminal rendering works |
| E: Tables | 6 | Table has valid projection; native two-pass rendering; all parity gates green |

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| `NodeKind` exhaustiveness breaks across 3 crates | Add `Section` in a single commit that updates all match arms + serde + validation simultaneously |
| Recursive component graphs | Projection depth guard with configurable limit and diagnostic on overflow |
| Darkmatter Flow A regressions | Every phase that adds native heading/list/table rendering re-runs `render_tree_parity.rs` |
| Table complexity | Table is last (Phase 6); lessons from simpler components inform the two-pass design |
| `Prose` styling loss in projection | Accepted as `DiagnosticKind::Lossy` in `Warn`; documented as known limitation; open question for post-group-1 |
| `Rc<dyn TerminalRenderable>` is `!Send`/`!Sync` | Projected `RenderNode` trees are owned and serializable — they can cross thread boundaries |

## Files Created/Modified Per Phase

### Phase 0

```
renderable/src/tree/node.rs              (modify — Section variant)
renderable/src/tree/attrs.rs             (modify — HintNamespace, typed helpers)
renderable/src/tree/validate.rs          (modify — Section rules)
renderable/src/tree/render/markdown.rs   (modify — Section arm)
renderable/src/tree/render/browser.rs    (modify — Section arm)
renderable/src/tree/render/mod.rs        (modify — CodeRenderer trait)
biscuit-terminal/lib/src/components/renderable.rs  (modify — render_tree_node)
biscuit-terminal/lib/src/render_tree/options.rs    (modify — context extensions)
biscuit-terminal/lib/src/render_tree/component.rs  (modify — hook wiring)
biscuit-terminal/lib/src/render_tree/projection.rs (new)
biscuit-terminal/lib/src/render_tree/browser_adapter.rs (new)
biscuit-terminal/lib/src/render_tree/mod.rs        (modify — new modules)
biscuit-terminal/lib/tests/parity_helpers.rs       (new)
```

### Phase 1

```
biscuit-terminal/lib/src/components/section.rs   (modify — render_tree_node)
biscuit-terminal/lib/src/render_tree/render.rs   (modify — native heading/section)
biscuit-terminal/lib/tests/section_parity.rs     (new)
```

### Phase 2

```
renderable/src/tree/attrs.rs                     (modify — ListRenderHints)
biscuit-terminal/lib/src/components/list.rs      (modify — render_tree_node)
biscuit-terminal/lib/src/render_tree/render.rs   (modify — native list rendering)
biscuit-terminal/lib/tests/list_parity.rs        (new)
```

### Phase 3

```
renderable/src/tree/attrs.rs                     (modify — CodeRenderHints)
renderable/src/tree/render/mod.rs                (modify — CodeRenderer)
biscuit-terminal/lib/src/render_tree/component.rs (modify — hook field)
biscuit-terminal/lib/src/render_tree/options.rs  (modify — hook field)
biscuit-terminal/lib/src/render_tree/render.rs   (modify — code hook check)
darkmatter/lib/src/markdown/yaml_block.rs        (modify — render_tree_node)
darkmatter/lib/tests/yaml_block_parity.rs        (new)
```

### Phase 4

```
renderable/src/tree/attrs.rs                     (modify — ProgressHints)
biscuit-terminal/lib/src/components/progress.rs  (modify — render_tree_node)
biscuit-terminal/lib/src/render_tree/render.rs   (modify — progress detection)
renderable/src/tree/render/markdown.rs           (modify — progress fallback)
renderable/src/tree/render/browser.rs            (modify — progress fallback)
biscuit-terminal/lib/tests/progress_parity.rs    (new)
```

### Phase 5

```
renderable/src/tree/attrs.rs                     (modify — ColumnsHints)
biscuit-terminal/lib/src/components/two_column.rs (modify — render_tree_node)
biscuit-terminal/lib/src/render_tree/render.rs   (modify — two-column rendering)
biscuit-terminal/lib/tests/two_column_parity.rs  (new)
```

### Phase 6

```
renderable/src/tree/attrs.rs                     (modify — table hint types)
biscuit-terminal/lib/src/components/table/table.rs (modify — render_tree_node)
biscuit-terminal/lib/src/render_tree/render.rs   (modify — native table)
biscuit-terminal/lib/tests/table_parity.rs       (new)
```

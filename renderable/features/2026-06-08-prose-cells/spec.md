---
status: ready for planning and implementation
reviewed: true
date: 2026-06-08
owner: ken
parent: renderable/features/_completed/2026-06-06-tree-closeout/spec.md
depends-on:
  - renderable/features/_completed/2026-06-06-tree-closeout/spec.md
origin: biscuit-terminal Table ergonomics gap (Prose cells)
---

# Prose Table Cells

Add a `StyledProse(Prose)` variant to `TableCellContent` so callers can place
styled, capability-aware inline content in a table cell without rendering it
to terminal bytes during construction.

The canonical render-tree path projects the `Prose` directly as structured
children of `NodeKind::TableCell`. Browser, Markdown, MarkdownPlus, and the
standard terminal renderer then use their existing shared folds. The retained
terminal cursor-alignment escape hatch resolves `StyledProse` through the same
`Terminal` that renders the surrounding table before width planning.

## Goal

```rust
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::table::{Table, TableColumn, TableCellContent};

let table = Table::new()
    .with_columns(vec![
        TableColumn::new("Status"),
        TableColumn::new("Owner"),
    ])
    .with_data(vec![vec![
        TableCellContent::from(Prose::new("<dim>inactive</dim>")),
        TableCellContent::from(Prose::new("<b>Alice</b>")),
    ]]);
```

This produces capability-appropriate terminal styling and the existing Prose
semantics on Browser, Markdown, and MarkdownPlus without requiring the caller
to select a target or terminal when constructing the cell.

## Motivation

`TableCellContent::Text` accepts strings containing terminal escape sequences,
and the table already measures and wraps those bytes correctly. Requiring
callers to pre-render `Prose` is nevertheless the wrong API boundary:

1. Pre-rendered bytes freeze terminal capability assumptions before the table
   is rendered.
2. Pre-rendered SGR can bypass a later no-color terminal profile.
3. Callers must manage resets correctly to avoid styling table padding or
   borders.
4. ANSI bytes are terminal-only and cannot preserve semantic emphasis, links,
   or richer style intent for Browser and Markdown targets.

`Prose` is already `tree render only`: it parses bracket tags and its Markdown
subset directly into `RenderNode` children, and all targets fold those nodes
through the shared renderers. Table cells should preserve that structure
rather than introduce a serialized Prose payload or another target-specific
walker.

## Design Decisions

### Structured children are authoritative

For `StyledProse`, the `TableCell` child nodes are the authoritative
cross-target representation. `TableCellHints` remain reconstruction metadata
for terminal table planning; they are not a serialization format for Prose
source or render trees.

This deliberately avoids:

- serializing `Prose` or `RenderNode` into `raw_value`;
- reconstructing the original Prose grammar after projection;
- a table-specific Markdown translation walker;
- rendering Prose to ANSI before tree projection.

### Embedded Prose is inline

A table cell is a phrasing-only render-tree container. `StyledProse` therefore
uses `Prose::to_render_nodes()` and ignores the `Prose` value's outer
`Layout`; column width, alignment, wrapping, and padding remain table-owned.

Fenced code blocks are the only block-level nodes Prose can produce. They are
not valid table-cell children. During cell projection, each top-level `Code`
node MUST degrade to escaped literal text containing its code body, while the
remaining inline nodes retain their structure. This keeps the infallible table
builder API, produces a valid tree on every target, and avoids silently
dropping content. The fence, language, and block layout are intentionally not
preserved inside a cell.

### Contextless display is not a render path

`Display for TableCellContent` remains available. For `StyledProse`, it uses
`Prose::render_optimistic(None)` as a compatibility fallback. Production
terminal rendering and width planning MUST NOT use this fallback: they use
either structured tree children or a cell resolved with the active `Terminal`.

## Scope

### 1. `TableCellContent` API

Add to `biscuit-terminal/lib/src/components/table/cell.rs`:

```rust
pub enum TableCellContent {
    Text(String),
    Integer(i64),
    Float(f64),
    Currency(Currency, f64),
    /// Inline Prose resolved by the active render target.
    StyledProse(Prose),
}
```

Add `From<Prose> for TableCellContent`. Existing conversions are unchanged.
`StyledProse` has string-column semantics: left alignment by default and the
column's normal string wrapping policy.

Update the enum documentation, examples, and variant count. Do not describe
literal escape sequences or glyph bytes in public docs.

### 2. Canonical tree projection

Update `Table::to_render_tree_node`:

- `Text`, `Integer`, `Float`, and `Currency` continue to project their
  formatted value as one `RenderNode::text` child.
- `StyledProse` projects the inline nodes from `Prose::to_render_nodes()`,
  applying the fenced-code degradation above.
- The cell hint kind is `"styled_prose"`.
- The hint `raw_value` is `null`; the structured child nodes carry the
  content. The original bracket-tag source is not duplicated into metadata.

Update `TableCellHints::kind` documentation to include `"styled_prose"`.

The table body slot style remains attached to the `TableCell` node. A Prose
child's local style composes inside that cell-level style under the existing
renderer rules.

### 3. Standard terminal tree path

The standard `TerminalRenderable for Table` path already renders cell children
through the active terminal context before reconstructing values for width
planning. For a `"styled_prose"` hint, `reconstruct_cell` MUST return
`TableCellContent::Text(rendered_text)`.

This is intentional. At this boundary the structured children have already
been resolved for the active terminal, and the native table planner needs the
resulting visible string. It does not need, and cannot losslessly recover, the
original Prose grammar.

Width measurement, wrapping, line splitting, slot styles, striping, and border
reset behavior then reuse the existing ANSI-aware `Text` machinery.

### 4. Cursor-alignment terminal escape hatch

`Table::render_bespoke` is retained for `prefer_cursor_alignment` on a TTY and
does not consume the canonical tree. Before any width or row-height planning,
it MUST clone or materialize the table data once, resolving every
`StyledProse` cell with `Prose::render(term)` into `Text`.

All subsequent bespoke planning and emission use that resolved data. Do not
render the same Prose cell independently in measurement, wrapping, and
emission; a single resolution step prevents inconsistent output and repeated
parsing.

`render_optimistic` follows the same rule with its optimistic `Terminal`.

### 5. Browser, Markdown, and MarkdownPlus

No table-specific Prose renderer is added.

- Browser receives semantic/styled Prose children inside `<td>`.
- Markdown uses the existing Prose/tree lowering: semantic emphasis and links
  remain Markdown, while unsupported presentation degrades according to the
  shared Markdown renderer.
- MarkdownPlus uses the existing richer style lowering where supported.
- Existing table-cell escaping for pipes, links, and line breaks remains the
  only table-specific escaping layer.

This preserves one target-neutral projection and prevents table output from
drifting from standalone Prose behavior.

### 6. Compatibility

This is an additive enum variant and conversion. Existing `Text` cells,
including caller-supplied ANSI strings, remain supported and retain their
current behavior.

Any exhaustive matches on `TableCellContent` inside the workspace must add the
new variant. No existing conversion changes meaning.

## Verification

### API and projection

- `Prose: Into<TableCellContent>` produces `StyledProse`.
- A styled cell projects semantic `Strong`, `Emphasis`, `Delete`, `Link`, or
  styled `Span` children rather than a text node containing Prose source or
  ANSI.
- The hint is `kind = "styled_prose"` with `raw_value = null`.
- A fenced code block degrades to text and the projected table passes render
  tree validation.
- Prose layout does not become nested cell layout.

### Terminal

- Dim, bold, color, and link content resolve using the supplied terminal
  capabilities.
- A terminal profile with `ColorDepth::None` emits no color/style SGR. Use an
  explicitly constructed terminal profile rather than mutating `NO_COLOR` in
  a parallel unit test.
- Multiline and wrapped cells are measured by visible width and do not bleed
  style into padding, separators, borders, or adjacent rows.
- Mixed `StyledProse`, `Integer`, `Float`, and `Currency` rows retain typed
  formatting and alignment.
- Standard tree rendering and the cursor-alignment bespoke path both preserve
  visible content and styling.
- Instrumented or focused tests verify that the bespoke path resolves each
  Prose cell once before planning.

### Browser and Markdown

- Browser preserves semantic emphasis, links, and supported style attributes
  inside the table cell.
- Portable Markdown matches standalone Prose semantics for bold, italic,
  strikethrough, links, color degradation, and significant-character
  escaping.
- MarkdownPlus matches standalone Prose's richer style behavior.
- Pipe characters and line breaks cannot corrupt the GFM table structure.

Existing table, Prose, and render-tree tests must continue to pass. No new
Level 2 terminal coverage is required because this feature introduces no new
terminal protocol or emulator-specific behavior; Level 1/unit coverage must
exercise both terminal table paths.

## Documentation

- Update `biscuit-terminal/lib/src/components/table/README.md` with the new
  variant and a two-column example.
- Update `.claude/skills/biscuit-terminal/components.md`, the authoritative
  local skill documentation, to mention Prose cells.
- Update `biscuit-terminal/docs/components/prose.md` to document inline Prose
  in table cells and the table-owned layout rule.
- Review affected rustdoc and inline comments for the old
  "every cell is one text child" contract and correct drift in the same
  change.

## Non-Goals

- Adding a Prose cell-header API. `TableColumn::header_prose` remains the
  header primitive.
- Supporting block-level Prose layout or fenced code blocks as blocks inside
  cells.
- Removing support for pre-styled ANSI in `Text`.
- Generalizing `TableCellContent` to arbitrary renderable components.
- Adding a Prose or render-tree serialization format to `TableCellHints`.
- Changing standalone Prose target semantics.

## Acceptance Criteria

1. `TableCellContent::from(Prose::new("..."))` produces `StyledProse`.
2. Canonical table projection stores Prose as valid structured cell children,
   with no ANSI or serialized Prose payload.
3. Standard terminal rendering and the cursor-alignment escape hatch resolve
   styled cells using the active terminal before width-sensitive work.
4. No-color, wrapping, multiline, striping, and border-reset behavior remain
   correct.
5. Browser, Markdown, and MarkdownPlus output reuse and match the shared Prose
   render-tree behavior.
6. Fenced code content degrades deterministically to valid inline text.
7. Existing `TableCellContent` behavior remains compatible, and all exhaustive
   matches handle the new variant.
8. Public docs, the authoritative skill documentation, and relevant code
   comments describe the implemented contract.

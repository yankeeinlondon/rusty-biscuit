---
status: ready for planning
date: 2026-06-04
owner: ken
parent: renderable/features/2026-06-04-css-box-architecture/spec.md
depends-on: renderable/features/2026-06-04-style-vocabulary/spec.md
---

# Tree Attrs — Typed Sparse Node Attributes, Inheritance, and the Perf Gate

The second chapter of the [CSS Box Architecture](../2026-06-04-css-box-architecture/spec.md).
It makes the render tree carry layout/style **cheaply and once**: typed sparse
node attributes (no per-node JSON round-trip), a single canonical inheritance
resolver, and a deterministic performance gate that prevents the cost from
creeping back as features grow. Depends on the
[style-vocabulary](../2026-06-04-style-vocabulary/spec.md) types (`Layout`,
`Style`, `Edges`, `Width`).

## Background

### The smell: attributes are stringly-typed JSON, (de)serialized per access

`renderable::tree::NodeAttrs` stores everything structured in one bag:

```rust
pub struct NodeAttrs {
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub data: BTreeMap<String, serde_json::Value>,   // ← Layout, Style, and 11 hint groups live here
}

pub fn layout(&self) -> Option<Layout> {
    let value = self.get_hint(HintNamespace::LAYOUT, "layout")?;   // format!("{ns}.{key}") → String alloc
    serde_json::from_value(value.clone()).ok()                     // Value clone + full deserialize
}
```

Every `node.layout()` / `node.style()` call during a render fold pays a
`format!`-built **key String allocation**, a **`serde_json::Value` clone** of
the whole subtree, and a **full serde deserialize**. The renderers call these
on every node, sometimes more than once. Thirteen typed accessors ride this
path today: `Layout`, `Style`, `SequenceJoin`, `ListMarkerPolicy`,
`ListRenderHints`, `CodeRenderHints`, `ProgressHints`, `ColumnsHints`,
`TaskHints`, `TableColumnHints` (keyed per column index), `TableCellHints`,
`TableTerminalHints`, and `table_title`.

This is the exact "features quietly erode performance" structure the
architecture program targets: the cost is invisible and scales with both tree
size and how many attrs each renderer reads.

### What stays sound

`Style::inherited_from` (text-appearance inheritance), the `Layout`/`Style`
types from style-vocabulary, and the `data` bag *as an escape hatch for genuine
extension* are all kept. The bag stops being the storage for first-class
attributes.

## Goals

- Replace JSON-bag storage of all 13 typed attributes with **typed fields** on
  `NodeAttrs`, so hot reads are `O(1)` with no allocation, clone, or serde.
- Keep the **common node small**: `Layout`/`Style` and the rare per-component
  hint groups are boxed; an unstyled node is a few null pointers.
- Provide **one canonical inheritance resolver** that every renderer threads,
  replacing per-renderer reimplementations.
- Add a **deterministic performance gate** that fails if the per-node JSON
  round-trip or per-node key allocation is reintroduced on the fold hot path.
- Preserve the vocabulary spec's **absence ≡ default** contract: a node with no
  layout/style/component attrs skips the styling pass.

## Non-Goals (deferred to sibling sub-specs)

- Target painting behavior — terminal/browser painting the padding box,
  honoring `fit-content`, lowering `padding`/`width` (→ *renderer-folds*).
- darkmatter's `style:`-frontmatter → attrs rewrite and the deletion of
  `LayoutContext` / `decorate`'s bespoke push-down (→ *darkmatter-cutover*).
  This spec provides the shared inheritance resolver; *darkmatter-cutover*
  switches darkmatter onto it.
- Any change to the `Layout`/`Style` field set (owned by style-vocabulary).

## The design

### 1. Typed, sparse `NodeAttrs`

```rust
pub struct NodeAttrs {
    pub id: Option<String>,
    pub classes: Vec<String>,

    /// Block geometry. `None` = no layout (skip the styling pass).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<Box<Layout>>,
    /// Paint. `None` = unstyled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<Box<Style>>,

    /// Hot per-node hints, stored inline (small enums).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_join: Option<SequenceJoin>,
    #[serde(default, skip_serializing_if = "ListMarkerPolicy::is_default")]
    pub list_marker_policy: ListMarkerPolicy,

    /// Rare per-component hint groups, boxed so the common node stays small.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<Box<ComponentHints>>,

    /// Arbitrary / rare extension data ONLY. No first-class attribute lives here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub data: BTreeMap<String, serde_json::Value>,
}

/// Per-component render hints. Only nodes of the matching kind carry one, so
/// the box is paid only where used.
pub enum ComponentHints {
    List(ListRenderHints),
    Code(CodeRenderHints),
    Progress(ProgressHints),
    Columns(ColumnsHints),
    Task(TaskHints),
    /// Table grouping — a table node may carry several at once.
    Table(TableHints),
    /// Cell-level hints live on individual cell nodes.
    TableCell(TableCellHints),
}

/// The table node's hints, grouped (column hints are keyed per column index).
pub struct TableHints {
    pub columns:  BTreeMap<usize, TableColumnHints>,
    pub terminal: Option<TableTerminalHints>,
    pub title:    Option<String>,
}
```

The existing accessor *surface* (`layout()`, `set_layout()`, `list_hints()`,
`progress_hints()`, …) is preserved so call sites change minimally, but each
accessor becomes a direct typed field read/write instead of a serde round-trip.
`set_hint`/`get_hint`/`remove_hint` remain only for `data` extension use.

### 2. Serialization — clean break, sparse on the wire

Typed fields use `#[serde(default)]` + `skip_serializing_if` so a node
serializes only the attributes it actually carries, and an absent field
deserializes to its default. **No migration** of the old
`data: {"renderable.layout.layout": …}` shape is provided: no render trees are
persisted, no consumer reads the namespace keys directly, and the in-process
`render_tree_roundtrip` test re-passes on the new typed shape. (If a future need
for durable trees appears, a one-version migration is its own task.)

### 3. One canonical inheritance resolver

Only the text-appearance fields inherit (`color`, `emphasis`); box-painting
(`background`, `border`) and geometry never do — unchanged from
`Style::inherited_from`. Today each renderer reimplements the push-down
(`biscuit-terminal/lib/src/render_tree/render.rs` threads an `effective`
style; darkmatter's `decorate` does its own).

This spec adds **one resolver in `renderable`** that threads the effective
inheriting style during a fold, e.g.:

```rust
/// The effective text-appearance carried into a child during a fold.
/// Box-painting and geometry are not part of inheritance.
pub struct InheritedStyle(/* effective color + emphasis */);

impl InheritedStyle {
    pub fn enter(&self, node_style: Option<&Style>) -> (InheritedStyle, Style) { /* … */ }
}
```

Renderers thread one `InheritedStyle` down the tree and call `enter` per node;
nodes stay sparse (only explicitly-set style is stored). *renderer-folds* and
*darkmatter-cutover* switch the terminal/browser/markdown folds and darkmatter
onto this resolver; this spec lands the resolver and moves
`biscuit-terminal`'s fold onto it as the reference consumer.

### 4. Performance gate — deterministic structural invariant

The enforced gate is a `#[test]`, not a timing budget (CI timing is noisy and
lets real regressions through). Behind a test-only instrumentation hook, the
hint round-trip path (`set_hint`/`get_hint` and any `serde_json` call) and the
key-String allocation are counted. The test:

1. builds a representative styled corpus (block + inline + a table + a list +
   code + nested inheritance),
2. folds it once per target,
3. asserts the hint-round-trip count and per-node key-allocation count are
   **zero** for the fold.

Because the typed fields make the hot accessors incapable of touching serde,
the invariant is largely type-enforced; the test guards against a future change
that re-routes a first-class attribute back through `data`. Criterion benches
(`biscuit-terminal/lib/benches/render_tree.rs`,
`darkmatter/lib/benches/render_tree.rs`, `renderable/benches/render.rs`) are
kept for trend visibility only.

### 5. Absence ≡ default short-circuit

A node whose `layout`, `style`, and `component` are all `None` (and
`list_marker_policy` is default, `sequence_join` is `None`) carries no
appearance: the renderer skips the styling pass. This is now a typed
`is_none()` check rather than a bag lookup, realizing the vocabulary spec's
contract with no per-node cost.

## Acceptance Criteria

1. `NodeAttrs` carries typed `layout: Option<Box<Layout>>`,
   `style: Option<Box<Style>>`, `sequence_join`, `list_marker_policy`, and
   `component: Option<Box<ComponentHints>>`; `data` holds no first-class
   attribute. `rg 'set_hint\(HintNamespace::(LAYOUT|STYLE|LIST|TABLE|CODE'` finds
   no first-class attribute writes outside `data`-extension tests.
2. The 13 former accessors (`layout`, `style`, `sequence_join`,
   `list_marker_policy`, `list_hints`, `code_hints`, `progress_hints`,
   `columns_hints`, `task_hints`, `table_column_hints`, `table_cell_hints`,
   `table_terminal_hints`, `table_title`) read/write typed fields and perform
   **no** `serde_json` call. A unit test asserts each round-trips through the
   typed field, not the bag.
3. `NodeAttrs` serializes sparsely (`skip_serializing_if`) and deserializes
   old-but-typed payloads via `#[serde(default)]`; the `render_tree_roundtrip`
   test passes.
4. A single `renderable` inheritance resolver exists and is the only inheritance
   implementation in `biscuit-terminal`'s terminal fold (the per-renderer
   push-down is gone there). darkmatter's switch is *darkmatter-cutover*.
5. The performance-gate test exists, folds the corpus per target, and asserts
   zero hint round-trips and zero per-node key allocations during the fold.
6. The whole workspace **compiles and its tests run**: the storage change's
   call-site fixes land in this coordinated change.
7. Local renderable skill/docs describe the typed `NodeAttrs`, the
   `ComponentHints` grouping, the inheritance resolver, and the perf-gate
   invariant; no doc claims attributes are stored as JSON in `data`.

## Risks

- **Accessor call-site blast radius.** Every `set_*`/typed-read site across
  `renderable`/`biscuit-terminal`/`darkmatter` is touched. Mitigation: keep the
  accessor *names/signatures* stable so most call sites are unchanged; the
  change is internal to the accessor bodies plus the struct. Compiler-driven
  fixes for the rest.
- **Table hint grouping.** `TableColumnHints` is index-keyed and a table node
  can carry columns + terminal + title at once; collapsing them into one
  `ComponentHints::Table(TableHints)` must preserve per-index addressing.
  Mitigation: `TableHints.columns: BTreeMap<usize, _>`, with a test that sets
  hints on columns 0 and 2 and reads them back independently.
- **Inheritance unification regressions.** Moving `biscuit-terminal`'s fold onto
  the shared resolver could shift text-appearance push-down. Mitigation:
  characterization tests on the existing terminal inheritance behavior before
  the switch; parity is a reference per the architecture spec.
- **Gate instrumentation leaking into release.** The counting hook must be
  test-only. Mitigation: gate it behind `#[cfg(test)]` / a test-only feature so
  release builds carry no counter.

## Related

- [`../2026-06-04-css-box-architecture/spec.md`](../2026-06-04-css-box-architecture/spec.md)
  — architecture overview, the unifying thesis, and the sequencing contract.
- [`../2026-06-04-style-vocabulary/spec.md`](../2026-06-04-style-vocabulary/spec.md)
  — the `Layout`/`Style`/`Edges`/`Width` types this spec stores.
- [`renderable/src/tree/attrs.rs`](../../../renderable/src/tree/attrs.rs) — the
  `NodeAttrs` bag and the 13 accessors this spec converts to typed fields.
- [`biscuit-terminal/lib/src/render_tree/render.rs`](../../../biscuit-terminal/lib/src/render_tree/render.rs)
  — the terminal fold whose inheritance push-down moves onto the shared
  resolver.

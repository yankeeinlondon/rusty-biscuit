# Tree Attrs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert `renderable::tree::NodeAttrs` from a JSON bag to **typed sparse fields** so render-tree reads cost no allocation/clone/serde, add one canonical inheritance resolver, and add a deterministic performance gate — keeping the workspace green.

**Architecture:** `Layout`, `Style`, the two hot per-node hints, and the rare per-component hint groups become typed fields on `NodeAttrs`; `data` is left only for package-local extension namespaces (`darkmatter.hr`, `darkmatter.li`). The 13 existing accessor *names/signatures* are preserved (so call sites are untouched) — only their bodies switch from serde round-trips to typed-field reads, plus new borrowed `*_ref` accessors for hot paths. A shared `renderable` inheritance resolver replaces `biscuit-terminal`'s bespoke `effective`-style threading. A test-only counter enforces "zero renderable-owned hint round-trips during a fold."

**Tech Stack:** Rust 2024, `serde` (+ `serde_json` round-trip tests), `insta` snapshots, the monorepo `cargo`/`just` tooling, `md hash` for skill hashes.

**Spec:** [`spec.md`](spec.md) (architect-reviewed). Read it first, especially the serde clean-break and perf-gate sections. Depends on [style-vocabulary](../2026-06-04-style-vocabulary/spec.md) being implemented (it provides `Layout`/`Style`/`Edges`/`Width`).

---

## File Structure

**Modified (renderable):**
- `renderable/src/tree/attrs.rs` — add serde to the 13 hint types; add `ComponentHints`/`TableHints`; add typed `NodeAttrs` fields; rewrite the 13 accessor bodies to typed-field access; add borrowed `*_ref` accessors; add a test-only hint-round-trip counter.
- `renderable/src/tree/validate.rs` — validate typed fields directly; reject stale `renderable.*` keys in `data`; keep placement rules.
- Create: `renderable/src/tree/inherit.rs` — the shared `InheritedStyle` resolver.
- `renderable/src/tree/mod.rs` — module wiring + re-export `InheritedStyle`, `ComponentHints`, `TableHints`.

**Modified (consumer):**
- `biscuit-terminal/lib/src/render_tree/render.rs` — `Writer.effective` threading moves onto `InheritedStyle`.

**Modified (docs/fixtures):**
- `.claude/skills/renderable/tree.md` (+ any attrs mention), `renderable/docs/layout-and-style.md` / tree docs; re-accept the JSON-surface `insta` snapshots in `darkmatter/lib/tests/render_tree_roundtrip.rs`.

**Baseline:**

- [ ] **Step 0: Confirm green start (style-vocabulary already landed)**

Run: `cargo build -p renderable -p biscuit-terminal -p darkmatter && cargo test -p renderable --no-run`
Expected: clean build + test binaries compile.

---

## Task 1: Add serde to the hint types (compact tokens preserved)

The 13 hint types currently derive **no** serde (they were hand-serialized into the bag). Plain data structs get a derive; the token enums implement serde via their existing `to_token`/`from_token` so the wire form stays the documented compact tokens.

**Files:**
- Modify: `renderable/src/tree/attrs.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` in `attrs.rs`:

```rust
    #[test]
    fn token_enums_serialize_as_compact_tokens() {
        // Token enums serialize to their `to_token()` string, not the variant name.
        assert_eq!(
            serde_json::to_value(SequenceJoin::Tight).unwrap(),
            serde_json::Value::String(SequenceJoin::Tight.to_token().to_string())
        );
        let back: SequenceJoin =
            serde_json::from_value(serde_json::to_value(SequenceJoin::Tight).unwrap()).unwrap();
        assert_eq!(back, SequenceJoin::Tight);
    }

    #[test]
    fn hint_structs_serde_roundtrip() {
        let list = ListRenderHints { bullet: Some("* ".into()), ..Default::default() };
        let back: ListRenderHints =
            serde_json::from_str(&serde_json::to_string(&list).unwrap()).unwrap();
        assert_eq!(list, back);

        let cols = TableColumnHints::default();
        let back: TableColumnHints =
            serde_json::from_str(&serde_json::to_string(&cols).unwrap()).unwrap();
        assert_eq!(cols, back);
    }
```

(Use the real default-constructible variants; adjust `SequenceJoin::Tight` to an actual variant name if it differs.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p renderable tree::attrs -- token_enums hint_structs`
Expected: FAIL — `the trait \`Serialize\` is not implemented` / `Deserialize`.

- [ ] **Step 3: Add serde to the plain data structs**

Append `Serialize, Deserialize` to the derive lines of: `TaskHints`, `ListRenderHints`, `CodeRenderHints`, `ProgressHints`, `ColumnsHints`, `TableColumnHints`, `TableCellHints`, `TableTerminalHints`. Example:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListRenderHints { /* unchanged fields */ }
```

- [ ] **Step 4: Implement serde for the token enums via their tokens**

For each token enum (`SequenceJoin`, `ListMarkerPolicy`, `TaskState`, `ColumnConditional`, `ColumnWidthKind`), add manual impls that delegate to the existing `to_token`/`from_token` so the compact format is preserved:

```rust
impl serde::Serialize for SequenceJoin {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.to_token())
    }
}
impl<'de> serde::Deserialize<'de> for SequenceJoin {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let token = String::deserialize(d)?;
        SequenceJoin::from_token(&token)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid SequenceJoin token: {token}")))
    }
}
```

Repeat the pattern for the other four. (`to_token` returns `&'static str` for these; `ColumnConditional::to_token` returns `String` per attrs.rs — use it directly.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p renderable tree::attrs`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add renderable/src/tree/attrs.rs
git commit -m "feat(renderable): add serde to render-tree hint types"
```

---

## Task 2: Add typed `NodeAttrs` fields + `ComponentHints`; switch accessors

This is the storage switch. Add the typed fields and the `ComponentHints` grouping, then rewrite each accessor body to read/write the typed field instead of the bag, so there is **one** storage.

**Files:**
- Modify: `renderable/src/tree/attrs.rs`
- Modify: `renderable/src/tree/mod.rs` (re-export `ComponentHints`, `TableHints`)

- [ ] **Step 1: Write the failing tests**

Add to `attrs.rs` tests:

```rust
    #[test]
    fn typed_layout_does_not_touch_data() {
        let mut attrs = NodeAttrs::default();
        attrs.set_layout(&crate::layout::Layout::default());
        assert!(attrs.layout.is_some());          // typed field set
        assert!(attrs.data.is_empty());           // NOT in the bag
        assert_eq!(attrs.layout(), Some(crate::layout::Layout::default()));
    }

    #[test]
    fn table_column_hints_keep_per_index_addressing() {
        let mut attrs = NodeAttrs::default();
        let c0 = TableColumnHints { min_width: Some(3), ..Default::default() };
        let c2 = TableColumnHints { max_width: Some(9), ..Default::default() };
        attrs.set_table_column_hints(0, &c0);
        attrs.set_table_column_hints(2, &c2);
        assert_eq!(attrs.table_column_hints(0), c0);
        assert_eq!(attrs.table_column_hints(2), c2);
        assert_eq!(attrs.table_column_hints(1), TableColumnHints::default());
        assert!(attrs.data.is_empty());
    }

    #[test]
    fn default_node_serializes_without_new_fields() {
        let json = serde_json::to_string(&NodeAttrs::default()).unwrap();
        // sparse: no layout/style/component/sequenceJoin keys, no data
        assert!(!json.contains("layout"));
        assert!(!json.contains("component"));
    }

    #[test]
    fn extension_hint_still_uses_data() {
        let mut attrs = NodeAttrs::default();
        attrs.set_hint(HintNamespace("darkmatter.hr"), "kind", serde_json::json!("solid"));
        assert!(attrs.data.contains_key("darkmatter.hr.kind"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p renderable tree::attrs -- typed_layout table_column_hints default_node`
Expected: FAIL — `no field \`layout\` on type \`NodeAttrs\`` (and `data` not empty, because accessors still write the bag).

- [ ] **Step 3: Add `ComponentHints` + `TableHints`**

In `attrs.rs` (above `NodeAttrs`):

```rust
/// The table node's hints, grouped (column hints are keyed per column index).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableHints {
    pub columns: BTreeMap<usize, TableColumnHints>,
    pub terminal: Option<TableTerminalHints>,
    pub title: Option<String>,
}

/// Per-component render hints; only nodes of the matching kind carry one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum ComponentHints {
    List(ListRenderHints),
    Code(CodeRenderHints),
    Progress(ProgressHints),
    Columns(ColumnsHints),
    Task(TaskHints),
    Table(TableHints),
    TableCell(TableCellHints),
}
```

Re-export both from `renderable/src/tree/mod.rs`.

- [ ] **Step 4: Add the typed fields to `NodeAttrs`**

```rust
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeAttrs {
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<Box<crate::layout::Layout>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<Box<crate::style::Style>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence_join: Option<SequenceJoin>,
    #[serde(default, skip_serializing_if = "ListMarkerPolicy::is_default")]
    pub list_marker_policy: ListMarkerPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<Box<ComponentHints>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub data: BTreeMap<String, serde_json::Value>,
}
```

Add the predicate used by `skip_serializing_if`:

```rust
impl ListMarkerPolicy {
    pub(crate) fn is_default(&self) -> bool { *self == ListMarkerPolicy::default() }
}
```

(Confirm `id`/`classes` keep their existing serde attrs; only add fields.)

- [ ] **Step 5: Rewrite the accessor bodies to typed-field access**

Switch each accessor pair from `set_hint`/`get_hint` to the typed field. The `data` bag is no longer touched for `renderable.*` hints. Targets:

| Accessor pair | Typed field |
|---|---|
| `set_layout` / `layout` | `self.layout: Option<Box<Layout>>` |
| `set_style` / `style` | `self.style: Option<Box<Style>>` |
| `set_sequence_join` / `sequence_join` | `self.sequence_join` |
| `set_list_marker_policy` / `list_marker_policy` | `self.list_marker_policy` |
| `set_list_hints` / `list_hints` | `component → ComponentHints::List` |
| `set_code_hints` / `code_hints` | `component → ComponentHints::Code` |
| `set_progress_hints` / `progress_hints` | `component → ComponentHints::Progress` |
| `set_columns_hints` / `columns_hints` | `component → ComponentHints::Columns` |
| `set_task_hints` / `task_hints` | `component → ComponentHints::Task` |
| `set_table_column_hints` / `table_column_hints` | `component → ComponentHints::Table(TableHints.columns[i])` |
| `set_table_cell_hints` / `table_cell_hints` | `component → ComponentHints::TableCell` |
| `set_table_terminal_hints` / `table_terminal_hints` | `component → ComponentHints::Table(TableHints.terminal)` |
| `set_table_title` / `table_title` | `component → ComponentHints::Table(TableHints.title)` |

Representative rewrites:

```rust
pub fn set_layout(&mut self, layout: &crate::layout::Layout) {
    self.layout = Some(Box::new(layout.clone()));
}
#[must_use]
pub fn layout(&self) -> Option<crate::layout::Layout> {
    self.layout.as_deref().cloned()
}
/// Borrowed accessor for renderer hot paths (no clone).
#[must_use]
pub fn layout_ref(&self) -> Option<&crate::layout::Layout> {
    self.layout.as_deref()
}

pub fn set_sequence_join(&mut self, join: SequenceJoin) {
    self.sequence_join = Some(join);
}
#[must_use]
pub fn sequence_join(&self) -> Option<SequenceJoin> { self.sequence_join }

pub fn set_list_marker_policy(&mut self, policy: ListMarkerPolicy) {
    self.list_marker_policy = policy;
}
#[must_use]
pub fn list_marker_policy(&self) -> ListMarkerPolicy { self.list_marker_policy }
```

For the `component`-backed groups, add a small private helper so each setter mutates the right variant without clobbering a co-resident table sub-hint:

```rust
impl NodeAttrs {
    fn table_hints_mut(&mut self) -> &mut TableHints {
        let needs_init = !matches!(self.component.as_deref(), Some(ComponentHints::Table(_)));
        if needs_init {
            self.component = Some(Box::new(ComponentHints::Table(TableHints::default())));
        }
        match self.component.as_deref_mut() {
            Some(ComponentHints::Table(t)) => t,
            _ => unreachable!("just initialized to Table"),
        }
    }
}

pub fn set_table_column_hints(&mut self, index: usize, hints: &TableColumnHints) {
    self.table_hints_mut().columns.insert(index, hints.clone());
}
#[must_use]
pub fn table_column_hints(&self, index: usize) -> TableColumnHints {
    match self.component.as_deref() {
        Some(ComponentHints::Table(t)) => t.columns.get(&index).cloned().unwrap_or_default(),
        _ => TableColumnHints::default(),
    }
}

pub fn set_list_hints(&mut self, hints: &ListRenderHints) {
    self.component = Some(Box::new(ComponentHints::List(hints.clone())));
}
#[must_use]
pub fn list_hints(&self) -> ListRenderHints {
    match self.component.as_deref() {
        Some(ComponentHints::List(h)) => h.clone(),
        _ => ListRenderHints::default(),
    }
}
```

Apply the same shape to `code_hints`, `progress_hints`, `columns_hints`, `task_hints`, `table_cell_hints`, `table_terminal_hints` (the last two route through `table_hints_mut()` for terminal/title). Add `*_hints_ref` borrowed variants where a renderer reads them on the hot path (`list_hints_ref`, `code_hints_ref`, `style_ref`).

- [ ] **Step 6: Run the renderable tests + existing accessor doctests**

Run: `cargo test -p renderable tree::attrs`
Expected: PASS — the new tests plus the existing accessor doctests/round-trip tests still pass against the typed storage.

- [ ] **Step 7: Build the dependents (accessor signatures unchanged → should be clean)**

Run: `cargo build -p biscuit-terminal -p darkmatter`
Expected: clean (the accessor surface is identical; only bodies changed). Fix any stray direct `.data` reads of `renderable.*` keys the compiler/clippy surfaces by routing them through the typed accessor.

- [ ] **Step 8: Commit**

```bash
git add renderable/src/tree/attrs.rs renderable/src/tree/mod.rs
git commit -m "feat(renderable): store NodeAttrs layout/style/hints as typed sparse fields"
```

---

## Task 3: Validate typed fields; reject stale `renderable.*` data keys

**Files:**
- Modify: `renderable/src/tree/validate.rs`

- [ ] **Step 1: Write the failing tests**

Add to `validate.rs` tests:

```rust
    #[test]
    fn stale_renderable_hint_key_in_data_is_an_error() {
        let mut doc = single_paragraph_doc();   // a minimal valid Document helper
        doc.root_mut().attrs.data.insert(
            "renderable.layout.layout".into(), serde_json::json!({}),
        );
        let report = validate(&doc);
        assert!(report.findings.iter().any(|f| f.message.contains("renderable.")));
    }

    #[test]
    fn darkmatter_extension_key_in_data_is_allowed() {
        let mut doc = single_paragraph_doc();
        doc.root_mut().attrs.data.insert(
            "darkmatter.hr.kind".into(), serde_json::json!("solid"),
        );
        let report = validate(&doc);
        assert!(report.findings.is_empty());
    }
```

(Use the existing test helpers in `validate.rs`; `single_paragraph_doc`/`root_mut` may need to match local helper names.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p renderable tree::validate -- stale_renderable darkmatter_extension`
Expected: FAIL (no such validation yet).

- [ ] **Step 3: Update validation**

In `validate.rs`:
- Keep the placement rules (`sequence_join()` Root-only, `task_hints()` ListItem-only, `table_title()` Table-only) — they now read typed fields and need no change.
- Replace the List-marker-policy `get_hint(HintNamespace::LIST, "marker_policy")` placement check with `node.attrs.list_marker_policy != ListMarkerPolicy::default()`.
- Delete the `check_hints` token-format validation (lines ~398–476): typed fields cannot hold malformed tokens, so this is dead.
- Add a check that rejects any `data` key whose namespace starts with `"renderable."` (a stale first-class hint), while allowing other namespaces (`darkmatter.*`, etc.):

```rust
for key in node.attrs.data.keys() {
    if key.starts_with("renderable.") {
        report.findings.push(error(
            format!("stale renderable-owned hint key in data: {key}; first-class attrs are typed fields"),
            span.clone(),
        ));
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p renderable tree::validate`
Expected: PASS (new + existing placement tests).

- [ ] **Step 5: Commit**

```bash
git add renderable/src/tree/validate.rs
git commit -m "feat(renderable): validate typed NodeAttrs; reject stale renderable.* data keys"
```

---

## Task 4: Shared inheritance resolver; move the terminal fold onto it

**Files:**
- Create: `renderable/src/tree/inherit.rs`
- Modify: `renderable/src/tree/mod.rs` (module + re-export)
- Modify: `biscuit-terminal/lib/src/render_tree/render.rs`

- [ ] **Step 1: Write the failing resolver tests**

Create `renderable/src/tree/inherit.rs`:

```rust
//! The single inheritance resolver threaded by every render fold.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{Color, Tailwind};
    use crate::layout::TargetValue;
    use crate::style::{Border, PerMode, Style, TextEmphasis};

    fn red() -> Style {
        Style {
            color: Some(TargetValue::universal(PerMode::universal(Color::Tailwind(Tailwind::Red500)))),
            ..Style::default()
        }
    }

    #[test]
    fn enter_threads_color_and_emphasis_only() {
        let root = InheritedStyle::root();
        // A node sets bold + red; the child context inherits both, box-painting is cleared.
        let mut styled = red();
        styled.emphasis = TextEmphasis { bold: true, ..Default::default() };
        styled.border = Some(Border::default());
        let (child_ctx, effective) = root.enter(Some(&styled));
        assert!(effective.color.is_some());
        assert!(effective.emphasis.bold);
        // border is applied to THIS node's effective style but does not inherit:
        let (_, grandchild_effective) = child_ctx.enter(None);
        assert!(grandchild_effective.color.is_some());      // color inherited
        assert!(grandchild_effective.emphasis.bold);        // emphasis inherited
        assert!(grandchild_effective.border.is_none());     // box-painting did NOT inherit
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p renderable tree::inherit`
Expected: FAIL — `cannot find type InheritedStyle`.

- [ ] **Step 3: Implement the resolver**

Prepend to `inherit.rs`:

```rust
use crate::style::Style;

/// The effective text-appearance carried into a child during a fold. Only
/// `color` and `emphasis` inherit; box-painting (`background`, `border`) and
/// geometry never do.
#[derive(Debug, Clone, Default)]
pub struct InheritedStyle {
    inherited: Style, // only color + emphasis are ever populated
}

impl InheritedStyle {
    /// The root context: nothing inherited yet.
    pub fn root() -> Self {
        Self { inherited: Style::default() }
    }

    /// Enter a node carrying `node_style`. Returns the context to thread into
    /// the node's children and the full effective `Style` to apply to the node
    /// itself (node's own box-painting + inherited text appearance).
    pub fn enter(&self, node_style: Option<&Style>) -> (InheritedStyle, Style) {
        let effective = match node_style {
            Some(s) => s.inherited_from(&self.inherited),
            None => self.inherited.clone(),
        };
        let child = InheritedStyle {
            inherited: Style {
                color: effective.color.clone(),
                emphasis: effective.emphasis,
                ..Style::default()
            },
        };
        (child, effective)
    }
}
```

Wire `mod inherit; pub use inherit::InheritedStyle;` into `renderable/src/tree/mod.rs`.

- [ ] **Step 4: Run resolver tests**

Run: `cargo test -p renderable tree::inherit`
Expected: PASS.

- [ ] **Step 5: Move `biscuit-terminal`'s fold onto the resolver**

In `biscuit-terminal/lib/src/render_tree/render.rs`, replace the `Writer.effective: Style` field and its manual threading (`std::mem::take` + reconstructing `Style { color, emphasis, ..default }`) with an `InheritedStyle`:
- `effective: Style` → `inherited: InheritedStyle` (init `InheritedStyle::root()`).
- In `render_styled`, replace the manual merge with `let (child_ctx, effective) = self.inherited.enter(node.attrs.style_ref());` then thread `child_ctx` to children and apply `effective` to this node.
- The `heading_effective(...).inherited_from(&self.effective)` site becomes `heading_effective(...).inherited_from(child-or-current inherited)` using the resolver's effective; preserve current behavior.
- Sub-`Writer` construction clones the `InheritedStyle` instead of the `Style`.

Keep behavior identical; the resolver encapsulates the same color+emphasis push-down.

- [ ] **Step 6: Run terminal tests**

Run: `cargo test -p biscuit-terminal render_tree`
Expected: PASS. If an inheritance snapshot shifts, confirm it is identical text appearance (the resolver is behavior-preserving); a real diff is a bug to fix, not to re-baseline.

- [ ] **Step 7: Commit**

```bash
git add renderable/src/tree/inherit.rs renderable/src/tree/mod.rs biscuit-terminal/lib/src/render_tree/render.rs
git commit -m "feat(renderable): add shared InheritedStyle resolver; adopt in terminal fold"
```

---

## Task 5: Performance gate (structural invariant)

**Files:**
- Modify: `renderable/src/tree/attrs.rs` (test-only counter hook)
- Create test: `renderable/tests/attrs_perf_gate.rs` (or a `#[cfg(test)]` module in `attrs.rs`)

- [ ] **Step 1: Add the test-only counter hook**

In `attrs.rs`, behind `#[cfg(test)]`, add a thread-local that `set_hint`/`get_hint`/`remove_hint` bump, classified by namespace:

```rust
#[cfg(test)]
thread_local! {
    /// (renderable_owned, extension) hint-access counts since last reset.
    pub(crate) static HINT_ACCESSES: std::cell::Cell<(u64, u64)> = const { std::cell::Cell::new((0, 0)) };
}

#[cfg(test)]
pub(crate) fn reset_hint_accesses() { HINT_ACCESSES.with(|c| c.set((0, 0))); }
#[cfg(test)]
pub(crate) fn hint_accesses() -> (u64, u64) { HINT_ACCESSES.with(std::cell::Cell::get) }

#[cfg(test)]
fn record_hint_access(ns: HintNamespace) {
    HINT_ACCESSES.with(|c| {
        let (r, e) = c.get();
        if ns.0.starts_with("renderable.") { c.set((r + 1, e)) } else { c.set((r, e + 1)) }
    });
}
```

Add `#[cfg(test)] record_hint_access(ns);` at the top of `set_hint`, `get_hint`, and `remove_hint`.

- [ ] **Step 2: Write the gate test**

```rust
    #[test]
    fn fold_does_zero_renderable_owned_hint_roundtrips() {
        // Build a styled corpus: block + inline + table + list + code + nested
        // inheritance + a darkmatter extension hint.
        let doc = styled_corpus_document();   // helper: see below

        super::reset_hint_accesses();
        // Fold once per target (markdown here; terminal/browser folds live in
        // their crates' gate tests that import the same helper).
        let _ = crate::tree::render::render_markdown_document(&doc, &Default::default());
        let (renderable_owned, _extension) = super::hint_accesses();

        assert_eq!(
            renderable_owned, 0,
            "the fold must not round-trip any renderable-owned hint through `data`"
        );
    }
```

Provide `styled_corpus_document()` building a small `Document` exercising layout, style (with inheritance), a list (marker policy + list hints), a table (column + title), a code block, and one `darkmatter.hr` extension hint, so the `_extension` count is non-zero (proving the counter works) while `renderable_owned` is zero.

- [ ] **Step 3: Run to verify it fails, then passes**

Run: `cargo test -p renderable fold_does_zero`
Expected: FIRST run may FAIL if any accessor still touches the bag — fix that accessor; then PASS.

- [ ] **Step 4: Mirror the gate in the terminal/browser fold crates (optional in this task)**

Add the analogous `renderable_owned == 0` assertion in `biscuit-terminal` using the same corpus helper, so each target's fold is gated. (Markdown gate above is the minimum AC #5 requires; terminal/browser may follow in *renderer-folds*.)

- [ ] **Step 5: Commit**

```bash
git add renderable/src/tree/attrs.rs
git commit -m "test(renderable): structural perf gate — zero renderable-owned hint roundtrips per fold"
```

---

## Task 6: Docs, skills, and serde-surface snapshots

**Files:**
- Modify: `.claude/skills/renderable/tree.md` (+ any `attrs`/`NodeAttrs` mention), `renderable/docs/layout-and-style.md` / tree docs
- Re-accept: `darkmatter/lib/tests/render_tree_roundtrip.rs` JSON-surface snapshots

- [ ] **Step 1: Update docs/skills**

Describe the typed `NodeAttrs` (layout/style/sequence_join/list_marker_policy/component/data), the `ComponentHints` grouping (incl. the index-keyed `TableHints.columns`), the `InheritedStyle` resolver, and the perf-gate invariant. State explicitly: render-tree JSON is **same-version** serde output (debug/inspection/persistence), **not** a promised cross-version durable format; `data` is for package-local extension namespaces only. Remove any claim that layout/style/hints are stored as JSON in `data`.

Run: `rg -n 'stored as JSON|data: BTreeMap|renderable\.layout\.layout' .claude/skills/renderable renderable/docs`
Expected: no stale storage claims remain.

- [ ] **Step 2: Re-accept the changed JSON-surface snapshots**

The `data`-keyed hint JSON is gone from the wire; the document-JSON-surface snapshots change (the rendered *Markdown* snapshots must NOT). Inspect and accept only the JSON-surface diffs:

Run: `cargo test -p darkmatter --test render_tree_roundtrip` then `cargo insta review`
Expected: only `document_json_surface` / NodeAttrs-shape snapshots differ (typed fields replace `data` keys); rendered-output snapshots are unchanged. Accept the JSON-surface diffs; investigate any rendered-output diff as a regression.

- [ ] **Step 3: Regenerate skill hashes**

For each edited skill file: `md hash .claude/skills/renderable/tree.md` (etc.) and update its `hash:` frontmatter.

- [ ] **Step 4: Commit**

```bash
git add .claude/skills/renderable renderable/docs darkmatter/lib/tests
git commit -m "docs(renderable): document typed NodeAttrs, ComponentHints, inheritance resolver, perf gate"
```

---

## Task 7: Whole-workspace verification

- [ ] **Step 1: Acceptance greps**

Run: `rg -n 'set_hint\(HintNamespace::(LAYOUT|STYLE|LIST|TABLE|CODE|TERMINAL|WIDGET_' renderable/src --type rust`
Expected: only `data`-extension/stale-input test code (no first-class writes).

- [ ] **Step 2: Build + test the affected crates**

Run: `cargo build --workspace && cargo test -p renderable -p biscuit-terminal -p darkmatter`
Expected: PASS. Rendered-output snapshots unchanged; only JSON-surface snapshots were re-baselined in Task 6.

- [ ] **Step 3: Final commit (if needed)**

```bash
git add -A
git commit -m "chore(renderable): finalize typed NodeAttrs migration"
```

---

## Self-Review Notes (for the executor)

- **Spec AC coverage:** AC1 (Task 2 + Task 7 grep), AC2 (Task 2 typed accessors + tests), AC3 (Task 2 serde + Task 6 snapshots + Task 3 stale-key rejection), AC4 (Task 4 resolver + terminal adoption), AC5 (Task 5 gate), AC6 (every task ends on a build/test), AC7 (Task 6 docs), AC8 (Task 3 validation + placement rules).
- **Smaller-than-expected blast radius:** no `NodeAttrs { … }` literals exist outside `attrs.rs`, and accessor signatures are preserved, so dependents mostly recompile untouched.
- **Type/name consistency:** `layout_ref`/`style_ref`/`*_hints_ref` borrowed accessors (Task 2) are what the terminal fold (Task 4) and gate corpus consume. `ComponentHints` variants and `TableHints.{columns,terminal,title}` are used identically in Task 2's accessors and tests.
- **Do not** silently migrate stale `renderable.*` `data` keys into typed fields (spec §2): they are validation errors (Task 3), not inputs.

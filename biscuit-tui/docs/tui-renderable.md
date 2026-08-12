# `TuiRenderable` — bridging renderable components into ratatui

`TuiRenderable` lets biscuit-terminal / darkmatter components that already
implement `TerminalRenderable` — `CodeBlock`, `Table`, `Prose`,
`UnorderedList`, `BlockQuote`, ... — render inside a ratatui application as
native `ratatui::text::Text`.

It lives in `biscuit-tui` behind the off-by-default **`renderables`** feature,
because `renderable` itself is deliberately ratatui-free (it is the neutral IR
that also feeds the Markdown and Browser targets, and biscuit-terminal depends
on it without ratatui). The trait therefore has to live where ratatui is
already a dependency.

```toml
[dependencies]
biscuit-tui = { path = "...", features = ["renderables"] }
```

```rust
use biscuit_tui::TuiRenderable;
use darkmatter::markdown::code_block::CodeBlock;
use ratatui::widgets::Paragraph;

// inside `terminal.draw(|frame| { ... })`
let text = CodeBlock::rust(src).to_tui_text(area.width);
frame.render_widget(Paragraph::new(text).scroll((scroll, 0)), area);
```

`to_tui_text(width)` returns owned, fully-styled `Text<'static>`. A `Text` is
itself a `Widget` (renders left-aligned, clipped); wrap it in a `Paragraph` to
get vertical scroll, alignment, or a surrounding `Block`. Do **not** enable
`Paragraph::wrap` for a `Table` — the component already laid the table out at
`width`, and ratatui re-wrapping would corrupt the columns.

## How Tier 0 works

A single blanket impl covers the entire catalog:

```rust
impl<T: TerminalRenderable> TuiRenderable for T {
    fn to_tui_text(&self, width: u16) -> Text<'static> {
        let ansi = self.render_optimistic(Some(u32::from(width)));
        ansi.into_text().unwrap_or_default()       // ansi-to-tui
    }
}
```

The component renders itself to ANSI at the requested width — reusing
biscuit-terminal's mature layout engine (table column planner, word wrap, list
indent) and darkmatter's syntect highlighting — and `ansi-to-tui` parses the
SGR escapes into ratatui `Line`/`Span`/`Style`. This is the same pattern
`claudine/cli` already uses ad hoc in its autocomplete detail pane; the trait
just formalizes it so it is not copy-pasted.

This is identical to how the established render targets work: a component is
the source of truth, and each target folds it. Tier 0 simply folds *through*
the terminal target's ANSI on the way to ratatui.

## Fidelity tiers

The tiers are **not** "the same thing with more polish." Tier 0 and Tier 1 both
produce a flat, static, styled `Text`; the boundary that matters is at Tier 2,
which crosses into stateful interaction and is a different API entirely.

| | Tier 0 — ANSI bridge | Tier 1 — native tree fold | Tier 2 — `StatefulWidget` |
|---|---|---|---|
| Mechanism | `render_optimistic` → ANSI → `ansi-to-tui` → `Text` | `render_tree_node()` → native `fold(RenderNode, w) -> Text` | implement `StatefulWidget` + `*State` + `HandleEvent` |
| Output | `Text<'static>` (flat styled lines) | `Text<'static>` (flat styled lines) | a live widget owning render **and** events |
| Layout | reuses biscuit-terminal's engine (baked into the string) | re-resolves geometry against a ratatui sink | owns geometry, or uses ratatui's `Table`/`List` |
| Interactivity | none (scroll via `Paragraph`) | none intrinsic; can emit a line→node map for selection overlays | real: select, click, edit, filter, expand |
| Build cost | ~30 LOC, one blanket impl | Code: small. Table: large (re-implements the planner) | large per component |
| Maintenance | ~zero; new components work for free | a 4th renderer to keep in parity with terminal/browser/markdown | full ownership, diverges from biscuit-terminal |

### Tier 0 — shipped

What it gets right: every `TerminalRenderable` works immediately, with correct
layout and syntax highlighting, and new components are covered automatically.

Known limitations (all acceptable for read-only display):

- **Static** — no in-widget selection. Vertical scroll comes free via
  `Paragraph::scroll`.
- **Width-baked + per-frame cost** — fold + parse run each draw; for a large
  code file, syntect re-highlights each frame. Cache by `(content_hash,
  width)` if it sits in a hot redraw loop.
- **Ragged background** — a code block's background paints under the glyphs
  only, not out to the panel's right edge. Cosmetic; Tier 1 fixes it by padding
  the last span to `width`.
- **No structural handle** — the flattened `Text` has lost "line 7 = table row
  3," so a row-selection cursor is not possible. Tier 1's fold can emit that
  map.
- **Links / images dropped** in the round-trip — irrelevant for `CodeBlock` and
  `Table`.

### Tier 1 — possible future work

A native `RenderNode` → `Text` fold, parallel to the existing terminal/browser
folds, mapping `renderable::style::Style` straight to `ratatui::style::Style`
with no ANSI round-trip. It would fix the ragged background, drop the parse
cost, and let the fold emit a line→node map for selection overlays.

It slots in **behind the unchanged `to_tui_text` seam** — the blanket impl
branches on `render_tree_node()` and falls back to the ANSI bridge for any
`NodeKind` not yet natively folded — so no call site changes. Because the
blanket impl forecloses per-type impls (coherence), the upgrade is grown inside
that one function, `NodeKind` by `NodeKind`, exactly like the terminal and
browser folds were grown.

Cost is uneven: `NodeKind::Code` has almost no geometry to re-resolve (cheap);
`Table` needs the column planner, which currently lives in biscuit-terminal and
emits strings, so a faithful native table fold is a substantial effort (or a
biscuit-terminal refactor to make its renderer generic over a `String` vs
`Text` sink).

**Do Tier 1 for a specific `NodeKind` only when Tier 0's limitations on that
kind actually bite** — e.g. the ragged code background in a bordered panel, or
syntect cost on huge files.

### Tier 2 — possible future work

Real interaction (scroll/select/click/edit/filter) means owning a render +
event lifecycle: a first-class biscuit-tui component (`StatefulWidget` +
`*State` + `HandleEvent`, like `ChooseOne` / `InputTable`). This is **not**
`TuiRenderable` at higher fidelity — `TuiRenderable` is static by nature.

The moment interaction is required, the bridge's value (reusing
biscuit-terminal's rendering) starts to collapse:

- **Interactive table** → prefer ratatui's built-in `Table` / `List` widgets
  (they already do selection, scroll, and highlight). Bridging biscuit-terminal's
  `Table` buys nothing once you need cell-level interaction.
- **Interactive code viewer** → compose: a Tier-1 `Code` fold for the
  highlighted spans, wrapped in a thin `StatefulWidget` that owns scroll /
  search / selection state. This reuses darkmatter's highlighting without a
  rewrite.

## Decision heuristic

- Read-only display, maybe scrolled → **Tier 0** (today).
- Read-only, but the ragged code background or syntect-per-frame cost bites →
  **Tier 1 for that `NodeKind`** (Code is cheap; Table is not).
- User scrolls + searches + selects lines → **Tier 1 Code fold + a thin
  `StatefulWidget`** (Tier 2 via composition).
- User edits, or selects/acts on table rows → **Tier 2**, and for tables reach
  for ratatui's native `Table` / `List` rather than the bridge.

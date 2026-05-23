# Design: Layout Visual Tests

**Date:** 2026-05-16
**Status:** Approved
**Crates:** `biscuit-terminal`, `darkmatter`

## Problem

Seven components are on the render-tree architecture (Section, UnorderedList,
TwoColumn, Progress, Table, BlockQuote in `biscuit-terminal`; YamlBlock in
`darkmatter`). Each carries a `Layout` (left/right/top/bottom margin,
alignment, row-fill, word-wrap). The bespoke `TerminalRenderable::render` path
honors that `Layout`; the tree path projects components into `RenderNode`s
folded by `render_terminal_node`. Whether layout settings survive the tree
projection is currently unverified — the `render_tree_parity` example already
exposed drift (TwoColumn not wrapping to width, Section spacing differences).

We need tooling to (a) visually inspect how layout settings behave across both
render paths and (b) lock current behavior against future regressions.

## Goals

- A runnable **inspection harness** showing each component under a matrix of
  layout settings, bespoke vs tree, side by side.
- **Snapshot tests** capturing the same matrix for regression protection.
- Harness and snapshot tests render through **one shared code path** so they
  never diverge.

## Non-Goals

- Fixing layout drift. Snapshots record current behavior *including* known
  bugs. Any fixes are separate follow-up work informed by this tooling.
- Browser/HTML or Markdown render targets. Terminal only.
- Color correctness. This is layout testing; snapshots strip ANSI.

## Architecture

Approach A — one shared support file per crate, included by both a harness
example and a snapshot test.

### File layout

`biscuit-terminal` (6 components):

| File | Role |
|------|------|
| `lib/tests/layout_matrix_support.rs` | Shared: `Scenario` list, per-component case builders, `render_pair()`, side-by-side formatter |
| `lib/examples/layout_matrix.rs` | Harness; includes support via `#[path = "../tests/layout_matrix_support.rs"] mod support;` |
| `lib/tests/layout_matrix.rs` | Snapshot test; includes support via `mod layout_matrix_support;` |

`darkmatter` (YamlBlock): the identical trio, scoped to one component. The
~25-line side-by-side formatter is duplicated rather than shared cross-crate —
cross-crate test-helper sharing is worse than the duplication.

### Shared support module

```rust
/// One cell of the matrix: a layout configuration applied at a width.
struct Scenario {
    name: &'static str,   // e.g. "left_margin_4"
    layout: Layout,       // full Layout to apply to the component
    width: u32,           // render width in columns
}

/// Returns the full scenario list (one dimension varied at a time).
fn scenarios() -> Vec<Scenario>;

/// A named component, with a closure that builds it under a scenario
/// and renders both paths.
struct ComponentCase {
    name: &'static str,
    /// Builds the component with `scenario.layout` applied, then returns
    /// (bespoke output, tree output) — both with ANSI retained.
    render: Box<dyn Fn(&Scenario) -> (String, String)>,
}

fn component_cases() -> Vec<ComponentCase>;
```

`render_pair` logic inside each `ComponentCase::render` closure:

- Build a fresh component, apply `scenario.layout` via `with_layout`.
- Bespoke: `component.render(&Terminal::new_optimistic(scenario.width))`.
- Tree: `component.render_tree_node()` (or `render_tree()` for BlockQuote) →
  `render_terminal_node(&node, &TerminalRenderOptions::new(&term, Warn))`.
- Return `(bespoke, tree)`.

BlockQuote implements `TreeRenderable::render_tree` (returns `RenderNode`); the
other six implement `TerminalRenderable::render_tree_node` (returns
`Option<RenderNode>`). Each component gets its own closure, so this difference
is handled per case.

## Scenario matrix

One dimension varied at a time from a default baseline. Full cross-product is
infeasible (thousands of cells); one-at-a-time keeps the matrix at ~13
scenarios.

| Scenario | Varies |
|----------|--------|
| `baseline` | default `Layout` at width 80 |
| `left_margin_4` | `left_margin = Margin::Cells(4)` |
| `right_margin_4` | `right_margin = Margin::Cells(4)` |
| `top_margin_2` | `top_margin = Margin::Cells(2)` |
| `bottom_margin_2` | `bottom_margin = Margin::Cells(2)` |
| `left_margin_pct_10` | `left_margin = Margin::Percent(10)` |
| `margin_auto` | `left_margin` and `right_margin` = `Margin::Auto` |
| `align_center` | `alignment = Alignment::Center` |
| `align_right` | `alignment = Alignment::Right` |
| `row_fill_alt` | one alternative `RowFill` strategy |
| `word_wrap_alt` | one alternative `WordWrap` policy |
| `width_40` | baseline layout, width 40 |
| `width_120` | baseline layout, width 120 |

Exact `Margin`, `RowFill`, and `WordWrap` variants are confirmed against the
`renderable::layout` API during implementation.

≈ 13 scenarios × 7 components ≈ 90 matrix cells.

## Harness

`cargo run -p biscuit-terminal --example layout_matrix`
`cargo run -p darkmatter --example layout_matrix`

For each `(component, scenario)`:

- Render both paths via the shared `render_pair`.
- Print side by side, **ANSI retained**, ANSI-aware column padding, headed
  `── Component × scenario ──────`.

Optional positional arg filters to a single component:
`cargo run --example layout_matrix -- table`. No arg renders the whole matrix
(pipe to a pager for review). No assertions — purely for human inspection.

## Snapshot tests

`cargo test -p biscuit-terminal --test layout_matrix`
`cargo test -p darkmatter --test layout_matrix`

For each `(component, scenario)`:

- Render both paths via the same `render_pair`.
- Strip ANSI from both (layout testing does not need color; stripped output
  diffs cleanly).
- Snapshot a stacked block — `BESPOKE\n{bespoke}\n---\nTREE\n{tree}` — via
  `insta::assert_snapshot!` with name `component__scenario`.

`insta` is already a dev-dependency. ~90 `.snap` files land under
`tests/snapshots/`. First run creates pending snapshots; they are reviewed and
accepted with `cargo insta review` / `cargo insta accept`.

## Error handling

- Case builder closures must construct components without panicking.
- Tree rendering errors are rendered into the output string as
  `<render error: {error}>` rather than panicking, so one failing cell does
  not abort the matrix.
- `render_pair` is the single rendering authority; harness and snapshot test
  share it verbatim.

## Testing

The snapshot test *is* the automated test for this feature — it is
self-validating once snapshots are accepted. The harness has no assertions.
Beyond that:

- Verify the harness runs for both crates and the component filter arg works.
- Verify `cargo test --test layout_matrix` passes after snapshot acceptance.
- Verify both crates remain `clippy`-clean.

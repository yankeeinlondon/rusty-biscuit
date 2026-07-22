---
status: draft
created: 2026-07-22
---

# Table Width Semantics

## Summary

`renderable::layout::Width` does not currently have an explicit `Fill` variant.
The shared contract documents `Auto` as filling a block's available width,
`FitContent` as hugging its content, and `Fixed(value)` as assigning an explicit
content-box width. `Table` now applies a different rule: `Auto` and
`FitContent` hug, while any `Fixed` value causes the final flexible column to
absorb slack up to the width handed to the table planner.

That table-specific behavior exposed an unresolved vocabulary problem. A fixed
width and a request to fill the parent are different policies. A fixed width
may happen to equal the parent width, such as `Fixed(100%)`, but it must not be
the vocabulary for fill behavior.

This specification proposes restoring that distinction with an explicit
`Width::Fill` variant. It is a draft: the currently failing terminal snapshot
test must not be accepted against either the old or new output until this
contract is ratified and implemented.

## Current State

The canonical enum in `renderable/src/layout/width.rs` is:

```rust
pub enum Width {
    Auto,
    FitContent,
    Fixed(TargetValue<Length>),
}
```

Its documented shared meanings are:

| Variant | Current shared contract |
|---|---|
| `Auto` | Fill the parent's available width; this is the default. |
| `FitContent` | Size the content box to the content's widest line. |
| `Fixed(value)` | Use an explicit cell, percentage, or target-specific width, clamped to the available width. |

The generic terminal tree renderer follows that contract. It resolves `Auto`
to the available content-box cap, resolves `Fixed(value)` to the authored value,
and measures `FitContent`. The browser renderer omits a CSS `width` declaration
for `Auto`, emits `width: fit-content` for `FitContent`, and emits the authored
width for `Fixed(value)`.

The current `Table` planner diverges from the shared `Auto` rule. Its
`apply_width_fill` helper returns unless the table layout is `Width::Fixed(_)`.
Consequently:

- a default or explicit `Auto` table hugs its columns;
- a `FitContent` table also hugs its columns;
- a `Fixed(60ch)` table expands its last flexible column until the table
  occupies the resolved 60-column content box;
- a `Fixed(50%)` table in an 80-column parent occupies 40 columns, not 80;
- a `Fixed(100%)` table occupies the parent width only because the authored
  fixed value resolves to that width.

Expanding internal columns to occupy an already-resolved fixed box is necessary
for a table to honor that explicit box size. It is not the same policy as
filling the parent. The helper name and comments currently blur that distinction
by referring to both operations as "fill."

There was formerly a type named `Fill`, removed by commit `755e6d49d`
(`refactor(renderable): rename Margin to Edges, add Width and padding, drop
Fill`). That type belonged to `Style.fill` and controlled painted backgrounds;
it was not a width-sizing variant. Its removal therefore does not provide a
replacement for an explicit `Width::Fill` policy.

## Behavior History

Commit `b66742909` (`feat(biscuit-terminal): make Width::Auto fill the available
width`) established the table rules that immediately preceded the regression
under review. At that point:

- `Width::Auto` filled the finite width handed to the table planner;
- `Width::Fixed(value)` also occupied the width handed to the planner, after
  the outer layout renderer resolved `value`;
- `Width::FitContent` hugged the table's natural content width;
- the final visible flexible column absorbed any remaining width;
- the layout-matrix snapshots were updated to record wide default/`Auto`
  tables.

Commit `5b1da6ac2` (`refactor(biscuit-terminal): default Width::Auto to hug;
Fixed fills`) changed those table-specific rules. It made `Auto` and
`FitContent` hug and restricted slack distribution to `Fixed(_)`. The commit
changed only `biscuit-terminal/lib/src/components/table/table.rs`; it did not
update the terminal layout-matrix snapshots that encoded the preceding `Auto`
fill behavior.

The change was intentional and has focused unit coverage, but it conflicts with
the canonical `Width::Auto` documentation and leaves `Fixed` carrying two ideas:
an explicit dimension and the only available signal that a table should consume
its assigned box.

## Failing Test Evidence

The reported test result is:

```text
TRY 4 FAIL [   0.837s] biscuit-terminal::layout_matrix layout_matrix_snapshots
```

The failure reproduces deterministically at the first mismatched snapshot,
`Table__baseline`. Both the public `render(&term)` path and the direct render-tree
path now produce the same compact table:

```text
┌──────┬───────┐
│ Name │ Score │
├──────┼───────┤
│ Ann  │ 42    │
│ Bob  │ 17    │
└──────┴───────┘
```

The committed snapshot expects both paths to span the 80-column scenario by
putting the slack in the `Score` column. Insta stops at the first mismatch. A
temporary regeneration outside the worktree showed that 17 terminal `Table`
snapshots differ under the current implementation; the `width_fit_content` and
`width_fixed_pct_50` snapshots already match.

This is not render-path drift: the two paths agree. It is a disagreement between
the stored `Auto` contract and the current table-width policy.

## Proposed Width Contract

Add an explicit sizing policy:

```rust
pub enum Width {
    Auto,
    Fill,
    FitContent,
    Fixed(TargetValue<Length>),
}
```

The proposed meanings are:

| Variant | Required behavior |
|---|---|
| `Auto` | Use the component's documented automatic sizing behavior. A table uses its intrinsic width; an ordinary block may use the full line according to its block contract. The result must remain consistent across terminal and browser renderers for the same node kind. |
| `Fill` | Occupy the parent's available content-box width after margins, padding, borders, and `max_width` are applied. |
| `FitContent` | Occupy the content's measured widest line, bounded by the available width and `max_width`. |
| `Fixed(value)` | Occupy exactly the resolved authored width, bounded by the available width and `max_width`. It never means "fill the parent" unless the authored value itself resolves to the parent's width. |

For tables, both `Fill` and `Fixed(value)` require distributing slack among
columns so the rendered grid occupies its resolved box. They differ in how that
box is selected: `Fill` selects the available parent box, while `Fixed(value)`
selects the authored dimension. The table helper should be renamed to describe
occupying a resolved box rather than treating `Fixed` as a synonym for fill.

Browser lowering should emit `width: 100%` for `Fill`, omit `width` for `Auto`,
emit `width: fit-content` for `FitContent`, and emit the resolved authored value
for `Fixed(value)`. Terminal lowering should make the same distinction in cells.
The Markdown renderer continues to ignore layout.

`Fill` must serialize as `"fill"`. Any Darkmatter frontmatter and CLI width
vocabulary must accept `fill` directly. Existing uses of `Fixed(100%)` that are
expressing policy rather than a literal authored percentage should migrate to
`Fill`; genuine percentage widths remain `Fixed(Percent(...))`.

## Layout-Matrix Changes

The matrix must independently lock all four policies:

- the baseline table records `Auto` and therefore the table's intrinsic width;
- the misleading `width_auto_fill` scenario is replaced by an explicit
  `width_fill` scenario using `Width::Fill`;
- `width_fit_content` remains content-hugging;
- `width_fixed_pct_50` continues to occupy half of the available parent width.

Snapshots must be reviewed, not accepted wholesale. In particular, the new
`Fill` snapshot should retain the wide shape formerly associated with `Auto`,
while `Fixed(50%)` must remain approximately 40 columns in an 80-column case.

## Temporary Test Suspension

`layout_matrix_snapshots` is temporarily ignored while this specification is
active. Its ignore reason points to this file so nextest and source inspection
retain an actionable warning. An enabled sentinel test has a warning-bearing
name and verifies that this active specification still exists, so the ordinary
nextest output remains visibly incomplete and moving or deleting the spec
without resolving the suspension fails explicitly. This is a narrow suspension:

- render-path parity remains enabled;
- browser and Markdown matrix snapshots remain enabled;
- focused table width-planning tests remain enabled;
- all non-table terminal snapshots remain present and unchanged.

The ignored test must be re-enabled in the same change that implements and
verifies the final width contract. The ignore marker must not be removed merely
by refreshing the 17 currently differing snapshots.

## Acceptance Criteria

- [ ] The width vocabulary expresses fill separately from an explicit fixed dimension.
- [ ] `Fixed(20ch)` occupies 20 content columns when at least 20 are available.
- [ ] `Fixed(50%)` occupies half of the resolved parent content box.
- [ ] `Fill` occupies the available parent content box and respects `max_width`.
- [ ] Table `Auto` behavior is explicitly documented and covered on terminal and browser targets.
- [ ] Table column planning distinguishes selection of the box from distribution within the box.
- [ ] Serde, Darkmatter frontmatter, and relevant CLI inputs support `fill`.
- [ ] The layout matrix has distinct `Auto`, `Fill`, `FitContent`, and `Fixed` coverage.
- [ ] The terminal snapshots are reviewed and updated according to the ratified contract.
- [ ] `layout_matrix_snapshots` is re-enabled and passes under nextest.
- [ ] The temporary warning sentinel is removed when the snapshot test is re-enabled.
- [ ] Documentation no longer describes `Fixed` as meaning "fill."
- [ ] The implementation and tests work on macOS, Windows, and Linux.

## Non-goals

- Restoring the removed background-painting `Style.fill` API.
- Treating `Fixed(100%)` as invalid; it remains a valid explicit percentage.
- Updating snapshots before the width contract is ratified.
- Changing Markdown output, which does not represent layout sizing.

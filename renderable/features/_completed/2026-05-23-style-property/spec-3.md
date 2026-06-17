---
status: ready for planning and implementation
date: 2026-05-23
owner: ken
parent: docs/rendering/style.md
sub-spec: 3-of-7
depends-on: spec-1.md (sub-spec #1), spec-2.md (sub-spec #2)
reviewed: true
---

# `style:` Frontmatter - Sub-Spec #3: Existing-Component Wiring

## Problem

Sub-spec #1 parses a typed `StyleFrontmatter` AST and sub-spec #2 wires
`style.page.*` to `DarkmatterPage`. This sub-spec wires the three non-list
component buckets that already have dedicated `PageComponent` variants:

- `style.table.*` -> `PageComponent::Tables`
- `style.images.*` -> `PageComponent::Images`
- `style.block-quote.*` -> `PageComponent::BlockQuotes`

After this sub-spec, the user's test document
`darkmatter/example-docs/rendering/style-prop.md` renders the table with
right alignment and a 50% max width. The remaining fixture gap is list styling,
which requires the `PageComponent::Lists` split in sub-spec #4.

## Goals

- Apply `CommonStyle` layout fields from `table`, `images`, and
  `block-quote` to the matching `PageComponent` via `DarkmatterPage`'s
  existing `use_alignment` and `with_fill` builders.
- Map `style.<bucket>.width` to `PageFill::Explicit(...)`.
- Map `style.<bucket>.max-width` to `PageFill::Max(...)`.
- Map `style.<bucket>.alignment` to `use_alignment(component, alignment)`.
- Preserve the precedence model from sub-spec #2: CLI values override
  frontmatter values at field level.
- Advance active style wiring to sub-spec `3` so the parser no longer emits
  `KnownButInactive { sub_spec: 3 }` warnings for the keys wired here.
- Use the same terminal and HTML application path so `md` and
  `md --output html` honor these component settings consistently.

## Non-Goals

- No new `PageComponent` variants. Lists are split and wired in sub-spec #4.
- No color or background-color application. Those remain sub-spec #5, even
  though the parser already accepts the fields.
- No HR migration. That remains sub-spec #6.
- No `page.stylesheet`, `page.meta`, `page.code`, hyperlinks, image
  `local-style`, or other bespoke knobs. Those remain sub-spec #7.
- No migration away from the deprecated `DarkmatterPage` page-layout storage.
  This sub-spec lowers `renderable::layout::Length` at the existing boundary.

## Dependencies

- Sub-spec #1 (schema/parser) merged.
- Sub-spec #2 (page wiring, `apply_page_style`, CLI override summaries,
  `--strict-style`, and active wiring phase support) merged.

## Design Decisions

1. **Width and max-width mapping.** `style.<bucket>.width` is a fixed target
   width and lowers to `PageFill::Explicit(unit)`. `style.<bucket>.max-width`
   is a cap and lowers to `PageFill::Max(unit)`. This deliberately matches
   the current `DarkmatterPage` fill model instead of inventing a second
   component layout channel.
2. **`renderable::layout::Length` lowers at the `DarkmatterPage` boundary.**
   Add a small helper in `darkmatter::style` for component fill conversion:
   `Length::Zero` and `Length::Ch(n)` lower to `WidthUnit::Fixed(u16)` using a
   saturating cast, and `Length::Percent(p)` lowers to
   `WidthUnit::Percent(p)`. `Length::Css(_)` is invalid for
   `DarkmatterPage` component fill and returns `StyleApplyError` before
   rendering.
3. **Width and max-width are mutually exclusive for these buckets.** If the
   same bucket sets both fields, return a clear `StyleApplyError` such as
   ``style.table.width and style.table.max-width are mutually exclusive``.
   The parent `docs/rendering/style.md` currently says both can be combined,
   but `DarkmatterPage` exposes a single `PageFill` slot per component and
   terminal rendering cannot represent both without changing that contract.
   Rejecting the ambiguous input is the least surprising behavior for this
   phase; a future layout-storage migration can revisit combined width and
   max-width semantics.
4. **Block-quote width applies to the whole rendered block quote.** For
   `style.block-quote.width` and `style.block-quote.max-width`, the fill value
   constrains the block quote component's rendered wrapper, including the
   quote prefix/border and body. Implementers should audit the existing
   block-quote terminal path because it pushes a component width while inside
   the block quote rather than using exactly the same post-render layout path
   as tables and images.
5. **Images share the same lowering and precedence as tables.**
   `style.images.alignment: center` centers each rendered image or fallback
   image marker within the page content width. `style.images.max-width` and
   `style.images.width` lower through the same fill helper as tables.
6. **CLI overrides are component-field specific, with global flags claiming
   every component.** `--alignment` claims alignment for all components;
   `--align-tables`, `--align-images`, and `--align-block-quotes` claim only
   their component. `--fill` claims width/max-width for all components;
   `--fill-tables`, `--fill-images`, and `--fill-block-quotes` claim only
   their component. A claimed CLI field suppresses the overlapping
   frontmatter field. Component frontmatter still overrides the page-level
   alignment broadcast from sub-spec #2 when no CLI alignment claims that
   component.
7. **Color-bearing fields stay inactive.** `table.color`, `table.bg-color`,
   `images.color`, `images.bg-color`, `block-quote.color`, and
   `block-quote.bg-color` continue to emit `KnownButInactive { sub_spec: 5 }`.
   Do not mark them active or silently drop them in this phase.

## Public API

```rust
// darkmatter::style - extended

/// CLI fields that already claimed component-level layout values.
///
/// Built by darkmatter-cli from `Cli` after applying the same global-then-
/// component-specific precedence as `apply_cli_layout_flags`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ComponentStyleOverrides {
    pub tables_alignment: bool,
    pub tables_fill: bool,
    pub images_alignment: bool,
    pub images_fill: bool,
    pub block_quotes_alignment: bool,
    pub block_quotes_fill: bool,
}

/// Apply parsed table/image/block-quote style onto a DarkmatterPage builder.
///
/// CLI overrides suppress frontmatter for overlapping component fields. The
/// returned page has active sub-spec #3 settings applied.
pub fn apply_component_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    overrides: ComponentStyleOverrides,
) -> Result<DarkmatterPage, StyleApplyError>;
```

`darkmatter/cli/src/output.rs:render_terminal_output` and `html_artifact`
call this helper after `apply_page_style` and before rendering:

```text
DarkmatterPage::new(...)
  -> apply_cli_layout_flags(...)
  -> apply_page_style(...)
  -> apply_component_style(...)
  -> render
```

The helper order intentionally allows component-specific frontmatter to
override `style.page.alignment` broadcasts from sub-spec #2 while still letting
CLI flags win over both.

## Implementation Notes

- Advance the active wiring phase to `3`; do not mutate `SchemaLeaf::sub_spec`.
  That field remains roadmap metadata.
- Reuse a single internal helper for `table`, `images`, and `block-quote`:
  read `CommonStyle`, validate `width`/`max-width` exclusivity, lower fill,
  apply fill if not CLI-claimed, then apply alignment if not CLI-claimed.
- Use canonical kebab-case paths in all errors and diagnostics:
  `style.block-quote.max-width`, not `style.block_quote.max_width`.
- Keep parse-time behavior unchanged for unknown and deprecated keys.
  `--strict-style` still only promotes schema warnings (`UnknownKey`,
  `Deprecated`) to errors; `KnownButInactive` remains informational.
- Update `darkmatter/docs/rendering/style.md` to state that `width` and
  `max-width` cannot be combined for `table`, `images`, and `block-quote`
  until the layout-storage model can represent both.

## Tests

1. **Table alignment from frontmatter** - `style.table.alignment: right` in a
   fixture makes the table right-aligned within the available content width.
2. **Table max-width** - `style.table.max-width: 50%` at an 80-column terminal
   renders the table using no more than 40 cells of horizontal space after page
   margin and padding are applied.
3. **Table width** - `style.table.width: 30ch` lowers to
   `PageFill::Explicit(WidthUnit::Fixed(30))`.
4. **Image alignment and max-width** - a synthetic document with an embedded
   image reference plus `style.images.{alignment,max-width}` applies the
   expected layout to both terminal fallback output and HTML selectors.
5. **Block-quote max-width** - `style.block-quote.max-width: 30ch` constrains
   the whole rendered block quote to the resolved component width.
6. **Width plus max-width exclusivity** - both fields in the same bucket return
   `StyleApplyError` before rendering.
7. **CLI component flag overrides** - `--align-tables right` with
   `style.table.alignment: left` keeps the CLI value.
8. **CLI global fill overrides** - `--fill max=60` with
   `style.table.max-width: 50%` keeps the CLI fill for tables, images, and
   block quotes.
9. **Page broadcast overridden by component frontmatter** -
   `style.page.alignment: centered` plus `style.table.alignment: right` makes
   tables right-aligned while other unclaimed components remain centered.
10. **Active wiring warnings** - table/image/block-quote width, max-width, and
    alignment no longer emit `KnownButInactive`; their color and bg-color keys
    still emit sub-spec #5 warnings.
11. **Integration with `style-prop.md`** - the rendered fixture visibly applies
    page-level settings from sub-spec #2 and table right-alignment plus 50%
    max-width from this sub-spec, without asserting on unstable ANSI details.

## Acceptance Criteria

- `md darkmatter/example-docs/rendering/style-prop.md` produces output where
  the table is right-aligned and capped at 50% max width.
- `md --output html darkmatter/example-docs/rendering/style-prop.md` emits
  matching table/image/block-quote layout CSS where applicable.
- All sub-spec #1 and #2 tests continue passing.
- Existing `apply_cli_layout_flags` behavior remains unchanged for documents
  without `style:` frontmatter.
- `KnownButInactive { sub_spec: 3 }` warnings no longer fire for keys wired
  here.
- Invalid component-level `Length::Css(_)` values and width/max-width
  conflicts fail with clear `StyleApplyError` messages before rendering.
- `darkmatter/docs/rendering/style.md` is updated for the live support status,
  kebab-case spelling, CLI-over-frontmatter precedence, and the
  width/max-width exclusivity rule for these buckets.

## Risks

- **Width-vs-fill semantic ambiguity.** `width` sounds like CSS width, but the
  current terminal contract stores one `PageFill` per component. Mitigation:
  reject width/max-width combinations in this phase and document the
  limitation.
- **Block-quote rendering path differs from tables/images.** Block quotes
  adjust component width while rendering nested content. Mitigation: add a
  focused regression test that inspects visible width and wrapping for
  top-level block quotes.
- **CLI precedence regression.** Global `--alignment` and `--fill` must claim
  all component fields. Mitigation: construct `ComponentStyleOverrides` from
  the CLI after shorthand expansion and add global-plus-component override
  tests.
- **Warning lifecycle drift.** If active wiring phase is not advanced to `3`,
  newly wired keys will still produce `KnownButInactive`. Mitigation:
  acceptance tests assert warning suppression for this phase.

## Open Questions

None. The review resolves the component-level design decisions needed for
planning and implementation. Future combined width/max-width support should be
handled as part of a broader layout-storage migration, not this sub-spec.

## Out-of-Spec

After sub-spec #3 lands, the user's fixture renders correctly except for
`ul.*` and `ol.*` styling, which is handled by sub-spec #4. Sub-specs #5-#7
then add color, HR migration, and bespoke knobs.

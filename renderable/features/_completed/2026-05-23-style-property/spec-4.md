---
status: ready for planning and implementation
date: 2026-05-23
owner: ken
parent: docs/rendering/style.md
sub-spec: 4-of-7
depends-on: spec-1.md (sub-spec #1), spec-2.md (sub-spec #2), spec-3.md (sub-spec #3)
reviewed: true
---

# `style:` Frontmatter — Sub-Spec #4: `ul` / `ol` / `li` Split + Wiring

## Problem

`DarkmatterPage` currently has a single `PageComponent::Lists` variant that
covers both ordered and unordered lists. The schema reserves three
distinct buckets — `ul`, `ol`, `li` — each with independent
`alignment`/`max-width`/`width`/`color`/`bg-color`, plus
`ul.left-margin` for indent. Wiring `style.{ul,ol,li}.*` therefore
requires:

1. Splitting `PageComponent::Lists` into three variants.
2. Updating the renderer to track which list kind it's emitting.
3. Wiring each frontmatter bucket to its own component without losing the
   existing CLI list affordances.
4. Adding an independent list-indent channel, because `PageFill` is a single
   slot per component and the fixture legitimately sets both
   `style.ul.left-margin` and `style.ul.max-width`.

This is the largest *structural* sub-spec — it changes a public enum
exposed by `darkmatter::layout`.

## Goals

- Split `PageComponent::Lists` into `Ul`, `Ol`, `Li`.
- Update every `DarkmatterPage` consumer (`apply_cli_layout_flags`,
  `LayoutContext`, list rendering passes) to honor the new variants.
- Wire `style.{ul,ol,li}.{alignment,max-width,width}` and
  `style.ul.left-margin` onto the new variants.
- `style.ul.left-margin: Nch` applies as a list-specific left indent that
  can coexist with `style.ul.width` or `style.ul.max-width`.
- Suppress `KnownButInactive { sub_spec: 4 }` warnings for keys wired
  here.
- Provide a backward-compat affordance: CLI `--align-lists` continues to
  apply to all three variants until a future deprecation.
- Keep existing downstream code using `PageComponent::Lists` compiling and
  behaving as a broadcast fallback where practical.

## Non-Goals

- No color/bg-color application (#5).
- No nested-list inheritance (`ul` inside `ol` etc.) beyond what the
  rendering engine already does for visual nesting.
- No new list-specific knobs beyond what's already in the schema.
- No migration of the deprecated page-layout primitives to
  `renderable::layout`; this sub-spec only adds the minimum bridge needed for
  list wiring.

## Dependencies

- Sub-spec #1 (schema), #2 (page wiring), #3 (existing-component wiring)
  all merged.

## Design Decisions

1. **Keep `PageComponent::Lists` as a deprecated compatibility variant.**
   Add `Ul`, `Ol`, and `Li`, keep `Lists` for one release cycle, and mark it:

   ```rust
   #[deprecated(note = "use PageComponent::{Ul, Ol, Li}")]
   Lists,
   ```

   `Lists` is a broadcast/fallback variant, not the renderer's primary
   component key after this sub-spec. New renderer code must use `Ul`, `Ol`,
   or `Li`; compatibility code may read `Lists` when a concrete list variant
   has no more-specific value.

2. **Concrete list variants drive rendering.** A top-level
   `Tag::List(None)` renders under `PageComponent::Ul`; a top-level
   `Tag::List(Some(_))` renders under `PageComponent::Ol`. `Tag::Item`
   rendering consults `PageComponent::Li` for item-body overrides. Nested
   lists inherit the existing visual nesting behavior; this sub-spec only
   applies page-component layout at the top-level list boundary unless the
   renderer already has a safe nested-component hook.

3. **`--align-lists` and `--fill-lists` remain broadcast flags.** Existing
   CLI flags must apply to `Ul`, `Ol`, and `Li` by writing all three concrete
   variants. Add granular `--align-ul`, `--align-ol`, `--align-li`,
   `--fill-ul`, `--fill-ol`, and `--fill-li` flags in the same CLI group.
   Granular flags override the broadcast list flags for their component.

4. **Frontmatter precedence remains CLI > frontmatter.** Follow the model
   from sub-specs #2 and #3. A global CLI component flag (`--alignment`,
   `--fill`) claims all list components; `--align-lists` / `--fill-lists`
   claim `Ul`, `Ol`, and `Li`; granular list flags claim only their concrete
   component.

5. **`width` and `max-width` remain mutually exclusive per bucket.** Match
   sub-spec #3's reviewed direction: if one of `ul`, `ol`, or `li` sets both
   `width` and `max-width`, fail early with `StyleApplyError` instead of
   choosing a winner. This does not apply to `ul.left-margin`, which is not a
   width/fill value.

6. **`ul.left-margin` uses an independent indent channel.** Do not lower
   `style.ul.left-margin` to `PageFill::Indent`, because that would overwrite
   `style.ul.max-width` in the same fixture. Add a narrowly-scoped
   `DarkmatterPage`/`LayoutContext` list-indent facility, e.g.
   `with_list_left_margin(PageComponent::Ul, WidthUnit)` and
   `LayoutContext::list_left_margin(PageComponent)`. It should accept only
   `PageComponent::Ul` in this sub-spec and return a clear apply error for
   `Ol`, `Li`, or non-list components. `PageFill::Indent` remains available
   for CLI fill semantics, but it is not the storage for `style.ul.left-margin`.

7. **Indent and width stacking order is fixed.** For unordered lists:
   resolve `ul.left-margin` first, subtract it from the available list body
   width, then apply `ul.width` or `ul.max-width` to the remaining body width,
   and finally apply alignment padding. This means `left-margin: 4ch` plus
   `max-width: 40` produces a 4-cell offset and a body wrapping at no more
   than 40 cells, capped by the remaining page width.

8. **Percent resolution bases.** `ul.left-margin: N%` resolves against the
   current page content width after page margin/padding and page max-width are
   known. `ul`/`ol`/`li` `width` and `max-width` lower through the same helper
   used by sub-spec #3: fixed `ch` values saturate to `u16`, percentages
   remain `WidthUnit::Percent(p)` and resolve in `LayoutContext` against
   content width, and `Length::Css(_)` fails with `StyleApplyError`.

9. **`li.*` applies to item bodies, not markers.** Markdown does not expose a
   stable independent marker box. `li.alignment`, `li.width`, and
   `li.max-width` affect each item's content/body after the marker prefix is
   emitted. Marker placement is governed by the containing `Ul`/`Ol`
   component. If `li` and the containing list both set alignment or width, the
   `li` value wins for the item body only.

10. **Browser selectors must split too.** `component_selectors` should map
    `Ul` to `ul`, `Ol` to `ol`, `Li` to `li`, and keep deprecated `Lists` as
    `ul, ol` for compatibility. Generated CSS order should emit `Lists`
    first, then concrete variants, so granular list styles win by normal CSS
    cascade when both are present.

## Public API (Sketch)

```rust
// darkmatter::layout — modified

pub enum PageComponent {
    Images,
    BlockQuotes,
    Tables,
    CodeBlocks,
    #[deprecated(note = "use PageComponent::{Ul, Ol, Li}")]
    Lists,
    Ul,
    Ol,
    Li,
}

impl PageComponent {
    /// Concrete list components in broadcast order.
    pub const LISTS: [PageComponent; 3] = [Self::Ul, Self::Ol, Self::Li];
}

// darkmatter::style — extended

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ListStyleOverrides {
    pub ul_alignment: bool,
    pub ul_fill: bool,
    pub ul_left_margin: bool,
    pub ol_alignment: bool,
    pub ol_fill: bool,
    pub li_alignment: bool,
    pub li_fill: bool,
}

pub fn apply_list_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    overrides: ListStyleOverrides,
) -> Result<DarkmatterPage, StyleApplyError>;
```

`apply_list_style` should reuse the lowering helper introduced by sub-spec #3
for `CommonStyle::{width,max_width,alignment}` and add only the `ul.left-margin`
lowering described above.

## Implementation Notes

- Update `PageComponent::ALL` to include only the concrete variants used by
  `use_alignment_for_all` / `with_fill_for_all`: `Images`, `BlockQuotes`,
  `Tables`, `CodeBlocks`, `Ul`, `Ol`, `Li`. Do not include deprecated
  `Lists` in all-component broadcasts.
- `LayoutContext::component_alignment(PageComponent::Ul|Ol|Li)` and
  `component_fill(PageComponent::Ul|Ol|Li)` should fall back to the deprecated
  `Lists` entry when the concrete component has no value. This preserves
  external code that still calls `use_alignment(PageComponent::Lists, ...)`.
- `darkmatter/cli/src/output.rs::apply_cli_layout_flags` should stop writing
  `PageComponent::Lists` for `--align-lists` / `--fill-lists` and instead
  write all concrete list variants.
- The terminal renderer should choose the list component before pushing
  component width/alignment. Item-body overrides should be scoped tightly so
  the marker prefix does not become part of the right/center-aligned body.
- The HTML renderer/page CSS path must use the split selectors above so
  `md --output html` matches terminal behavior for list-level wiring.
- Advance `ACTIVE_STYLE_WIRING_SUB_SPEC` to `4` after sub-spec #3 and this
  sub-spec are both active. If sub-spec #3 is still draft when implementation
  begins, keep the implementation ordered so #3's fields are not accidentally
  suppressed without being wired.

## Tests

1. **ul left-margin** — fixture's `style.ul.left-margin: 4ch` → each
   top-level unordered list item body starts 4 cells further right than it
   would without the style.
2. **ul max-width** — fixture's `style.ul.max-width: 40` → bullet body
   wraps at 40 cells.
3. **ul left-margin + max-width coexist** — fixture's
   `style.ul.left-margin: 4ch` and `style.ul.max-width: 40` both apply in the
   same render; neither setting overwrites the other.
4. **ol alignment** — fixture's `style.ol.alignment: right` → ordered
   list contents right-aligned.
5. **li body alignment** — `style.li.alignment: right` aligns item body text
   while preserving the marker prefix placement.
6. **li independent of ul/ol** — `style.li.color: red-500` (when #5
   lands) applies regardless of which list type the item is in.
7. **`--align-lists` broadcast** — `--align-lists right` applies to all
   three variants when no frontmatter specifies them.
8. **`--align-ul` granular** — overrides only `PageComponent::Ul`.
9. **deprecated Lists fallback** — a library test using
   `page.use_alignment(PageComponent::Lists, PageAlignment::Right)` still
   affects both unordered and ordered lists when no concrete override exists.
10. **width + max-width exclusivity** — `style.ul.width` plus
   `style.ul.max-width` returns `StyleApplyError`; same for `ol` and `li`.
11. **browser selectors** — generated page CSS uses `ul`, `ol`, and `li`
    selectors separately for concrete variants.
12. **active wiring warnings** — `ul.width`, `ul.max-width`,
    `ul.alignment`, `ul.left-margin`, `ol.width`, `ol.max-width`,
    `ol.alignment`, `li.width`, `li.max-width`, and `li.alignment` no longer
    emit `KnownButInactive { sub_spec: 4 }`; list color keys still emit their
    future sub-spec warning until #5 lands.

## Acceptance Criteria

- The fixture renders with `ul` left-margin (4ch), ul max-width (40), and
  `ol` right-alignment.
- Existing code that uses `PageComponent::Lists` still compiles, emits a
  deprecation warning, and works as a fallback broadcast for list alignment
  and fill.
- `PageComponent::ALL` and all new code use `Ul`, `Ol`, and `Li` rather than
  the deprecated `Lists` variant.
- `style.ul.left-margin` coexists with `style.ul.width` or
  `style.ul.max-width`; it is not stored as `PageFill::Indent`.
- `width` and `max-width` conflict detection exists for `ul`, `ol`, and `li`.
- All previous sub-spec tests still pass.
- `KnownButInactive { sub_spec: 4 }` warnings suppressed for wired keys.
- `md --output html darkmatter/example-docs/rendering/style-prop.md` applies
  list styles through split list selectors, not the legacy `ul, ol` selector
  only.
- Documentation in `darkmatter/docs/rendering/style.md` is updated to use
  canonical kebab-case for list keys and to describe `ul.left-margin` stacking.

## Risks

- **Public-API breaking change.** Anything outside `darkmatter` that
  matches exhaustively on `PageComponent` will still need to handle new
  variants. Keeping deprecated `Lists` avoids removing a variant but cannot
  prevent non-exhaustive-match compile errors. Mitigation: document the
  change and update all workspace matches in the same patch.
- **Renderer pass complexity.** The list renderer must now know which
  variant it's inside. The pulldown-cmark events distinguish
  `Tag::List(None)` (ul) vs `Tag::List(Some(start))` (ol); use that.
- **`li` ambiguity.** If `style.li.alignment` conflicts with the
  containing `ul`/`ol` alignment, define a precedence rule (li wins for
  its own body; ul/ol governs marker placement).
- **Indent/fill interaction.** The existing `PageFill` model cannot represent
  indent plus max-width at the same time. Mitigation: use the independent
  list-indent channel required by this spec and add a fixture regression test.
- **CSS cascade drift.** Deprecated `Lists` selectors and concrete selectors
  can both target the same elements. Mitigation: emit deprecated broadcast CSS
  before concrete CSS and test the generated order.

## Reader Note

The reviewed spec intentionally changes the draft's `ul.left-margin` mapping.
The original draft lowered `style.ul.left-margin` to `PageFill::Indent`, but
that conflicts with `style.ul.max-width` because `DarkmatterPage` stores only
one `PageFill` per component. The implementation must add a separate
list-indent channel so the user's fixture can apply both values
simultaneously.

## Open Questions

None. The review settled the list split, compatibility, `li` semantics, and
indent/width stacking model for this sub-spec.

## Out-of-Spec

After sub-spec #4 lands, the user's `style-prop.md` fixture is fully
honored. Sub-specs #5–#7 add color, HR migration, and bespoke knobs.

---
status: draft
date: 2026-05-23
owner: ken
parent: docs/rendering/style.md
sub-spec: 3-of-7
depends-on: spec.md (sub-spec #1), spec-2.md (sub-spec #2)
---

# `style:` Frontmatter — Sub-Spec #3: Existing-Component Wiring

## Problem

Sub-spec #2 wires `style.page.*` to `DarkmatterPage`. This sub-spec
extends wiring to the three component buckets that *already* have a
`PageComponent` variant in `DarkmatterPage`:

- `style.table.*` → `PageComponent::Tables`
- `style.images.*` → `PageComponent::Images`
- `style.block-quote.*` → `PageComponent::BlockQuotes`

After this sub-spec, the user's test doc (`style-prop.md`) renders the
table at right-alignment + 50% max-width — completing one half of the
remaining bug (the other half — `ul.*` and `ol.*` — needs the lists split
in sub-spec #4).

## Goals

- Apply `CommonStyle` fields from each of `table`, `images`,
  `block-quote` onto the corresponding `PageComponent` via the existing
  `use_alignment` / `with_fill` builders.
- Map `style.<bucket>.width` → `PageFill::Explicit(...)`.
- Map `style.<bucket>.max-width` → `PageFill::Max(...)`.
- Map `style.<bucket>.alignment` → `use_alignment(component, alignment)`.
- Reuse the precedence model from sub-spec #2 (CLI > frontmatter).
- Suppress `KnownButInactive { sub_spec: 3 }` warnings for keys wired
  here.

## Non-Goals

- No new `PageComponent` variants (lists split is #4).
- No color/bg-color application (#5).
- No HR / hyperlinks / page.code / local-style (#7).
- No `images.local-style` (#7).

## Dependencies

- Sub-spec #1 (schema/parser) merged.
- Sub-spec #2 (page wiring + `apply_page_style` + `--strict-style`) merged.

## Decisions to Settle (Brainstorm Inputs)

1. **Map `style.<bucket>.width` exactly how?** `DarkmatterPage` exposes:
   - `PageFill::Full` (default)
   - `PageFill::Pad(WidthUnit)` (left+right padding)
   - `PageFill::Indent(WidthUnit)` (one-sided indent)
   - `PageFill::Max(WidthUnit)` (max-width cap)
   - `PageFill::Explicit(WidthUnit)` (fixed width)

   The user-facing `width` (fixed) and `max-width` (cap) map cleanly to
   `Explicit` and `Max`. Confirm.

2. **`WidthUnit` lowering bridge.**
   - `renderable::layout::Length::Ch(u32)` →
     `darkmatter::layout::WidthUnit::Fixed(u16)`. Saturating cast.
   - `Length::Percent(f32)` → `WidthUnit::Percent(f32)`.
   - Add a helper in `darkmatter::style::lowering` for this conversion.

3. **What happens if both `width` and `max-width` are set?** Today
   `DarkmatterPage::with_fill` accepts one `PageFill` per component. Decide:
   - Error: "width and max-width are mutually exclusive".
   - Width wins (most specific).
   - Last-one-applied wins.
   Recommended: error early so the user fixes the doc.

4. **What does `style.block-quote.width: 50%` actually mean visually?**
   Block-quotes currently render with a left border bar plus indented body.
   A fixed `Explicit(50%)` would constrain the body to half-width. Confirm
   this is the desired behavior (vs. ignoring `width` for block-quotes
   entirely).

5. **Images: same precedence + lowering as tables?** Yes, but confirm.
   `style.images.alignment: center` should center each image relative to
   the content width.

## Public API (Sketch)

```rust
// darkmatter::style — extended

pub fn apply_component_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    cli_overrides: &CliLayoutOverrides,
) -> Result<(DarkmatterPage, Vec<StyleWarning>), StyleApplyError>;
```

CLI integration: `apply_component_style` is called after
`apply_page_style` in `render_terminal_output` and `html_artifact`.

## Tests

1. **Table alignment from frontmatter** — `style.table.alignment: right`
   in fixture → table rendered with right alignment within the available
   content width.
2. **Table max-width** — `style.table.max-width: 50%` at 80-col terminal
   → table rendered at ≤40 cells of horizontal space.
3. **Image alignment + max-width** — synthetic doc with embedded image
   reference + `style.images.{alignment, max-width}` → matched output.
4. **Block-quote width** — `style.block-quote.max-width: 30ch` →
   block-quote body wraps at ≤30 cells.
5. **`width` + `max-width` exclusivity** — both set → error.
6. **CLI flag overrides** — `--align-tables right` AND
   `style.table.alignment: left` → CLI wins.

## Acceptance Criteria

- `md darkmatter/example-docs/rendering/style-prop.md` produces output
  where the table is right-aligned at 50% max-width.
- All sub-spec #1 and #2 tests continue passing.
- Existing `apply_cli_layout_flags` behavior unchanged.
- `KnownButInactive { sub_spec: 3 }` warnings suppressed for wired keys.

## Risks

- **Width-vs-fill semantic ambiguity.** Need to nail the `Explicit` vs
  `Max` distinction in docs to avoid the bug where a user sets `width:
  50%` and is surprised the table doesn't ALWAYS render at 50% (e.g.,
  the content might be narrower).
- **Block-quote rendering may need wider changes** if the body's intrinsic
  rendering ignores the fill width. Audit the block-quote rendering path
  before committing to a design.
- **Image alignment** depends on the image rendering subsystem already
  honoring `PageComponent::Images` alignment, which it does via
  `apply_cli_layout_flags`. Re-verify.

## Open Questions

1. Width vs. max-width exclusivity: error or precedence rule?
2. `style.block-quote.width` semantic — body width or wrapper width?
3. Does `style.images.color` (a future #5 concern) make sense at all?
   Probably not — flag it as `NotApplicable` rather than `KnownButInactive`.

## Out-of-Spec

After sub-spec #3 lands, the user's fixture renders correctly except for
the `ul.*` and `ol.*` styling — that needs sub-spec #4.

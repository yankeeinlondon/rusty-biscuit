---
status: draft
date: 2026-05-23
owner: ken
parent: docs/rendering/style.md
sub-spec: 4-of-7
depends-on: spec.md (sub-spec #1), spec-2.md (sub-spec #2), spec-3.md (sub-spec #3)
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
3. Wiring each frontmatter bucket to its own component.

This is the largest *structural* sub-spec — it changes a public enum
exposed by `darkmatter::layout`.

## Goals

- Split `PageComponent::Lists` into `Ul`, `Ol`, `Li`.
- Update every `DarkmatterPage` consumer (`apply_cli_layout_flags`,
  `LayoutContext`, list rendering passes) to honor the new variants.
- Wire `style.{ul,ol,li}.{alignment,max-width,width}` and
  `style.ul.left-margin` onto the new variants.
- `style.ul.left-margin: Nch` → `PageFill::Indent(WidthUnit::Fixed(N))`
  on `PageComponent::Ul`.
- Suppress `KnownButInactive { sub_spec: 4 }` warnings for keys wired
  here.
- Provide a backward-compat affordance: CLI `--align-lists` continues to
  apply to all three variants until a future deprecation.

## Non-Goals

- No color/bg-color application (#5).
- No nested-list inheritance (`ul` inside `ol` etc.) beyond what the
  rendering engine already does for visual nesting.
- No new list-specific knobs beyond what's already in the schema.

## Dependencies

- Sub-spec #1 (schema), #2 (page wiring), #3 (existing-component wiring)
  all merged.

## Decisions to Settle (Brainstorm Inputs)

1. **`PageComponent::Lists` deprecation strategy.** Keep the variant as
   `#[deprecated(note = "use Ul / Ol / Li")]` for one release cycle? Or
   remove outright since this is a fresh feature?
   Recommended: deprecate with `#[allow(deprecated)]` shims so existing
   external users (the CLI `--align-lists` flag, downstream test code)
   continue to compile.

2. **CLI flag handling for `--align-lists`.** It currently maps to a
   single `PageComponent::Lists`. Options:
   - Keep flag, apply to all three new variants.
   - Add three new flags (`--align-ul`, `--align-ol`, `--align-li`) and
     keep the old flag as a "broadcast" shortcut.
   - Same for `--fill-lists`.
   Recommended: keep `--align-lists` as broadcast; add granular flags.

3. **`style.li.*` semantics.** Does `style.li.alignment: right` align
   each *list item's content*, or just the item marker? Markdown lists
   don't have a natural concept of right-aligned list items. Either:
   - Wire it the same as paragraph alignment within the item body.
   - Reject it as nonsensical (`UnknownKey` upgrade in the descriptor).
   Lean toward (a) — accept it but document that it's most useful for
   nested-content alignment.

4. **`ul.left-margin` resolution timing.** Same as page percent (#2 has
   the helper) — resolve `Length::Percent` against terminal width at
   apply time.

5. **Width-vs-Indent semantic.** `style.ul.left-margin: 4ch` is an
   indent, not a fill. `style.ul.max-width: 40` is a fill. Both can
   coexist: indent applies first, then max-width caps the remaining
   width. Document.

## Public API (Sketch)

```rust
// darkmatter::layout — modified

pub enum PageComponent {
    Images,
    BlockQuotes,
    Tables,
    CodeBlocks,
    // Replaces `Lists`:
    Ul,
    Ol,
    Li,
}

#[deprecated(note = "use PageComponent::{Ul, Ol, Li}")]
impl PageComponent {
    /// Compatibility constant — equals `[Ul, Ol, Li]`.
    pub const LISTS: [PageComponent; 3] = [Self::Ul, Self::Ol, Self::Li];
}

// darkmatter::style — extended

pub fn apply_list_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    cli_overrides: &CliLayoutOverrides,
) -> Result<(DarkmatterPage, Vec<StyleWarning>), StyleApplyError>;
```

## Tests

1. **ul left-margin** — fixture's `style.ul.left-margin: 4ch` → each
   unordered list item indented by 4 cells.
2. **ul max-width** — fixture's `style.ul.max-width: 40` → bullet body
   wraps at 40 cells.
3. **ol alignment** — fixture's `style.ol.alignment: right` → ordered
   list contents right-aligned.
4. **li independent of ul/ol** — `style.li.color: red-500` (when #5
   lands) applies regardless of which list type the item is in.
5. **`--align-lists` broadcast** — `--align-lists right` applies to all
   three variants when no frontmatter specifies them.
6. **`--align-ul` granular** — overrides only `PageComponent::Ul`.

## Acceptance Criteria

- The fixture renders with `ul` left-margin (4ch), ul max-width (40), and
  `ol` right-alignment.
- `PageComponent::Lists` variant either removed or shims successfully
  with `#[deprecated]`.
- All previous sub-spec tests still pass.
- `KnownButInactive { sub_spec: 4 }` warnings suppressed for wired keys.

## Risks

- **Public-API breaking change.** Anything outside `darkmatter` that
  matches exhaustively on `PageComponent::Lists` will break. Search the
  workspace before deciding to remove vs. deprecate.
- **Renderer pass complexity.** The list renderer must now know which
  variant it's inside. The pulldown-cmark events distinguish
  `Tag::List(None)` (ul) vs `Tag::List(Some(start))` (ol); use that.
- **`li` ambiguity.** If `style.li.alignment` conflicts with the
  containing `ul`/`ol` alignment, define a precedence rule (li wins for
  its own body; ul/ol governs marker placement).

## Open Questions

1. `--align-lists` semantics — broadcast or removed?
2. `style.li.alignment` semantics — accept or reject?
3. Width-vs-indent stacking — document precise interaction.

## Out-of-Spec

After sub-spec #4 lands, the user's `style-prop.md` fixture is fully
honored. Sub-specs #5–#7 add color, HR migration, and bespoke knobs.

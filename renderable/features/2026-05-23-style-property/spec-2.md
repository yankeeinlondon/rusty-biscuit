---
status: ready for planning and implementation
date: 2026-05-23
owner: ken
parent: docs/rendering/style.md
sub-spec: 2-of-7
depends-on: spec-1.md (sub-spec #1)
reviewed: true
---

# `style:` Frontmatter — Sub-Spec #2: Page-Level Wiring

## Problem

Sub-spec #1 ships a parser that produces a `StyleFrontmatter` AST. The
`md` CLI doesn't yet *use* it — every parsed key whose wiring phase is greater
than the active wiring phase still emits a `KnownButInactive` warning. This
sub-spec wires the **page-level** subset of the AST (`style.page.*`) into the
`DarkmatterPage` builders that
`darkmatter/cli/src/output.rs:render_terminal_output` and `html_artifact`
already use, so that `md style-prop.md` finally honors page margins, padding,
max-width, alignment, and page-background settings.

This is the first sub-spec that produces a **user-visible behavior change**
for the originally reported bug, partially (page-level only; component-level
keys land in #3 and #4).

## Goals

- Read `style.page.*` from the parsed `StyleFrontmatter` and apply each
  setting onto a `DarkmatterPage` before rendering.
- Define a clear precedence rule between **frontmatter style** and
  **CLI flags** (`--margin`, `--padding`, etc.): CLI flags override
  frontmatter because invocation-time flags are more explicit than document
  defaults.
- Introduce `md --strict-style` CLI flag that promotes schema-validation
  warnings (`UnknownKey`, `Deprecated`) to errors. `KnownButInactive` does
  not fail strict mode.
- Emit `tracing` events for parsed-but-unwired keys so `RUST_LOG=darkmatter=info`
  surfaces what was found but not yet applied.
- Suppress `KnownButInactive { sub_spec: 2 }` warnings for the keys this
  sub-spec wires — they have moved from "inactive" to "active" and shouldn't
  generate informational noise.

## Non-Goals

- No component-level wiring (table/images/block-quote/lists — #3, #4).
- No color/bg-color application (#5).
- No HR migration (#6).
- No new bespoke knobs (#7).
- No changes to the schema shape or value deserializers produced by sub-spec
  #1. The only parser change is warning emission now that page-level keys are
  active.
- No migration away from `DarkmatterPage`'s deprecated page-layout storage.
  This sub-spec lowers `renderable::layout::Length` at the boundary and leaves
  the broader page-layout replacement to a separate feature.

## Dependencies

- **Sub-spec #1** must be merged. This sub-spec reads its `StyleFrontmatter`
  type and `from_frontmatter` / `into_strict` functions.

## Design Decisions

1. **CLI flags override frontmatter.** The integration order is:
   `DarkmatterPage::new(...)` → existing `apply_cli_layout_flags` →
   `darkmatter::style::apply_page_style(page, &style, overrides)`.
   `apply_page_style` receives a CLI-agnostic override summary and applies
   only frontmatter fields not claimed by CLI flags. This preserves the
   existing CLI shorthand precedence (`--margin` → `--mx` / `--my` →
   side-specific flags) and avoids coupling the library crate to
   `darkmatter-cli`.
2. **Override detection is field-level after CLI shorthand expansion.**
   `--margin 4` claims all four page margin sides; `--mx 4` claims left and
   right; `--ml 4` claims only left. Padding follows the same rule. `--max-width`,
   `--page-bg`, and `--alignment` each claim their corresponding page-level
   field.
3. **`--strict-style` semantics.** The flag is a library passthrough:
   `from_frontmatter` parses, and `into_strict` converts only schema warnings
   (`UnknownKey`, `Deprecated`) to errors. `KnownButInactive` remains
   informational and never fails strict mode.
4. **Warning suppression uses an active wiring phase, not descriptor mutation.**
   Keep `SchemaLeaf::sub_spec` as the roadmap phase that wires the key. Add an
   `ACTIVE_STYLE_WIRING_SUB_SPEC` constant (value `2` for this sub-spec), and
   emit `KnownButInactive` only when `leaf.sub_spec >
   ACTIVE_STYLE_WIRING_SUB_SPEC`. This is a deliberate correction to the
   earlier idea of changing `sub_spec` to `1`: mutating the descriptor would
   erase useful roadmap metadata and, with the current parser shape, would
   still emit `KnownButInactive { sub_spec: 1 }` unless the parser changed too.
5. **Length lowering at the `DarkmatterPage` boundary.**
   - `Length::Zero` lowers to `0`.
   - `Length::Ch(n)` lowers to `u16::try_from(n).unwrap_or(u16::MAX)`.
   - `Length::Percent(p)` resolves at apply time with rounded cell counts and
     the same `0.0..=100.0` validation already enforced by the parser.
   - `Length::Css(_)` is invalid for page-level terminal layout and returns a
     `StyleApplyError`.
6. **Percent base widths.** Horizontal margins and padding resolve against the
   captured terminal width. `style.page.max-width: N%` resolves against the
   content width after final page margin and padding values are known, including
   any CLI overrides. The resolved value is passed to
   `DarkmatterPage::with_max_width`; `0%` is rejected because
   `DarkmatterPage` treats `max_width = 0` as invalid.
7. **`page.background` maps directly.**
   `style.page.background: transparent|subtle|pronounced` maps to
   `PageBackground::{Transparent, Subtle, Pronounced}` and then to
   `DarkmatterPage::with_page_background`.
8. **`page.alignment` broadcasts to page components.**
   `DarkmatterPage` has no whole-page alignment container. For this sub-spec,
   `style.page.alignment` means "default alignment for every page component"
   and lowers through `use_alignment_for_all`. Component-specific style added
   in later sub-specs overrides this broadcast for its own component unless a
   CLI component-specific alignment flag claims that component.

## Public API

```rust
// darkmatter::style — new

/// CLI fields that already claimed page-level layout/style values.
///
/// Constructed by darkmatter-cli from `Cli` after applying the same shorthand
/// expansion rules as `apply_cli_layout_flags`. This type lives in the
/// darkmatter library so the style applicator does not depend on CLI argument
/// structs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PageStyleOverrides {
    pub margin_top: bool,
    pub margin_right: bool,
    pub margin_bottom: bool,
    pub margin_left: bool,
    pub padding_top: bool,
    pub padding_right: bool,
    pub padding_bottom: bool,
    pub padding_left: bool,
    pub max_width: bool,
    pub background: bool,
    pub alignment: bool,
}

/// Apply parsed page-level style onto a DarkmatterPage builder.
///
/// CLI overrides suppress frontmatter for overlapping fields. The returned
/// page has active `style.page.*` settings applied. Warnings remain owned by
/// the parser; suppression is handled by the active wiring phase.
pub fn apply_page_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    overrides: PageStyleOverrides,
) -> Result<DarkmatterPage, StyleApplyError>;
```

`darkmatter/cli/src/output.rs:render_terminal_output` and `html_artifact`
both call this helper before invoking `page.render(md)` /
`page.render_to_browser(md)`.

## CLI Changes

`darkmatter/cli/src/args.rs` adds:

```rust
/// Promote schema-validation warnings (unknown / deprecated keys) to errors.
#[arg(long)]
pub strict_style: bool,
```

`darkmatter/cli/src/output.rs` reads the parsed frontmatter via
`darkmatter::style::from_frontmatter`, logs non-fatal warnings, applies
`into_strict` when `--strict-style` is set, calls the existing
`apply_cli_layout_flags`, then calls `apply_page_style` with
a `PageStyleOverrides` value constructed in `darkmatter-cli` from `cli`.

For both terminal and HTML artifacts, the parse/strict/apply sequence must be
shared so `md --output html style-prop.md` and terminal rendering see the same
frontmatter behavior.

## Reader Note

This reviewed spec intentionally differs from the earlier brainstorm in two
places:

- It does not change `SchemaLeaf::sub_spec` to mean "already wired." That field
  remains the planned wiring phase, and the parser compares it with
  `ACTIVE_STYLE_WIRING_SUB_SPEC`.
- It applies CLI flags before page style, then uses an override summary to skip
  overlapping frontmatter fields. That gives percent `max-width` access to the
  final CLI-claimed margin/padding values without making frontmatter override
  the CLI.

## Tests

1. **Page margin from frontmatter** — `style.page.left-margin: 2ch` →
   rendered output starts every non-empty content line with two columns of
   leading whitespace. Snapshot or invariant test.
2. **CLI flag overrides frontmatter** — both `--margin-left 4` and
   `style.page.left-margin: 2ch` → CLI wins.
3. **Percent margin resolves against terminal width** —
   `style.page.left-margin: 10%` at 80-col terminal → 8 columns left margin.
4. **Percent max-width resolves after margins/padding** —
   `style.page.left-margin: 10%`, `style.page.right-margin: 10%`, and
   `style.page.max-width: 50%` at 100 columns → max width resolves to 40.
5. **`--strict-style` fails on unknown key** — exit non-zero with the
   `StyleWarning` details on stderr.
6. **`--strict-style` succeeds on schema-clean document** — only
   `KnownButInactive` warnings → exit 0.
7. **Page alignment broadcasts** — `style.page.alignment: centered`
   makes table/image/list/code-block/block-quote default alignment centered
   unless a component-specific CLI flag overrides it.
8. **Active wiring warnings** — after this sub-spec, page fields no longer emit
   `KnownButInactive`; `table`, `ul`, `ol`, and other future-phase keys still
   do.
9. **Integration with `style-prop.md`** — extend the existing
   `style_frontmatter.rs` integration test (or add a new one alongside) that
   asserts the rendered page output has the expected margins/padding/
   max-width without the integration test asserting on terminal-specific
   ANSI bytes.

## Acceptance Criteria

- `md darkmatter/example-docs/rendering/style-prop.md` produces output where
  the page-level margins (2ch left, 4ch right, 1 row top, 0 rows bottom)
  are visibly applied. Render-snapshot test confirms.
- `md --output html darkmatter/example-docs/rendering/style-prop.md` uses the
  same page-level frontmatter values through `render_to_browser`.
- Existing `cargo nextest run -p darkmatter` passes.
- No regression on `apply_cli_layout_flags` users (existing CLI flag
  behavior unchanged).
- Documentation in `darkmatter/docs/rendering/style.md` updated to reflect
  page-level support is live, uses canonical kebab-case keys, and documents
  CLI-over-frontmatter precedence.
- `KnownButInactive { sub_spec: 2 }` warnings no longer fire for keys
  wired here.
- Invalid page-level CSS lengths and zero resolved `max-width` values fail with
  clear `StyleApplyError` messages before rendering.

## Risks

- **Precedence regression.** If we get the CLI-vs-frontmatter rule wrong,
  existing CLI users could see surprising changes when a doc happens to
  include `style:` block. Mitigation: end-to-end CLI test asserting CLI
  flags still win unchanged.
- **`Length::Percent` resolution timing.** Resolving at apply time requires
  the terminal width to be known. `DarkmatterPage::new(&term)` already
  captures it, so this is fine — but document the contract.
- **Page `max-width` percent ambiguity.** A percentage max width could be
  interpreted against raw terminal width or post-margin content width. This
  spec chooses post-margin/post-padding content width because it matches
  `LayoutContext` component-fill semantics and makes page max-width compose
  predictably with margins.
- **Warning lifecycle drift.** If future phases forget to advance
  `ACTIVE_STYLE_WIRING_SUB_SPEC`, valid newly-wired keys will keep producing
  `KnownButInactive`. Mitigation: every sub-spec acceptance test must assert
  warning suppression for its keys.
- **Pre-existing double-alias gap** from sub-spec #1's final review has already
  been addressed by the current walker (`block_quote` container alias recurses
  as `block-quote`). Keep a regression test in place rather than handling this
  in sub-spec #2.

## Open Questions

None. The review settled the known page-level design gaps for this sub-spec.

## Out-of-Spec

After sub-spec #2 + #3 + #4 land, the user's original bug is fully fixed.
After #5 lands, color/bg-color works. After #6, HR migration is complete.
After #7, the last bespoke knobs work.

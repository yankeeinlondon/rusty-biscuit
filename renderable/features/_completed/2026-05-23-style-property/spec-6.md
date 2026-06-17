---
status: ready for planning and implementation
date: 2026-05-23
owner: ken
parent: docs/rendering/style.md
sub-spec: 6-of-7
depends-on: spec-1.md (sub-spec #1), spec-2.md (sub-spec #2), spec-3.md (sub-spec #3), spec-4.md (sub-spec #4), spec-5.md (sub-spec #5)
reviewed: true
---

# `style:` Frontmatter - Sub-Spec #6: HR Migration

## Problem

Horizontal rules have a legacy styling path that is separate from the
`style:` frontmatter AST. The current implementation reads page-wide defaults
from top-level `hr:` in
`darkmatter/lib/src/markdown/block/hr_builder.rs` and reads per-rule
attribute overrides from `--- { style: waves }` in
`darkmatter/lib/src/markdown/block/rule_processor.rs`. The `style:`
frontmatter contract reserves `style.hr.kind` plus the common fields for HR
styling, but those keys are still parsed only as known-but-inactive schema
leaves until this sub-spec lands.

`darkmatter/docs/rendering/style.md` explicitly flags this:

> **IMPORTANT:** currently the `hr` functionality has implemented their
> bespoke styles directly to the top-level `hr` property and that needs
> to be moved here as `style.hr`

This sub-spec executes that migration and renames the per-rule HR pattern key
from `style` to `kind` so the inline form matches the frontmatter schema:

- Old inline syntax: `--- { style: waves }`
- New inline syntax: `--- { kind: waves }`

## Goals

- Read page-wide HR defaults from `style.hr.*` instead of top-level `hr:`.
- Keep top-level `hr:` as a deprecated alias for one release cycle, with a
  `Deprecated { replacement: "style.hr" }` warning.
- Rename the inline HR-attribute key `style` to `kind`.
- Keep inline `style` as a deprecated alias for one release cycle, with a
  `Deprecated { replacement: "kind" }` warning.
- Replace `HrStyle.kind: Option<String>` with a typed enum that matches the
  `biscuit_terminal::components::horizontal_rule::RuleStyle` taxonomy.
- Add a typed `HrWeight` enum and `style.hr.weight` so the migration does not
  drop the existing page-wide `hr.weight` behavior.
- Wire `style.hr.{kind,width,max-width,alignment,weight,color,bg-color}` to
  terminal and browser HR rendering.
- Suppress `KnownButInactive { sub_spec: 6 }` warnings for every HR key wired
  here.

## Non-Goals

- No new HR kinds beyond the existing `RuleStyle` variants.
- No SVG/image HR feature expansion beyond what `HorizontalRule` already
  supports.
- No global redesign of `HorizontalRule`; this sub-spec only changes how
  Darkmatter supplies defaults and overrides.
- No support for combining `style.hr.width` and `style.hr.max-width`; match
  sub-specs #3 and #4 by rejecting the ambiguous combination.

## Dependencies

- Sub-spec #1 (schema/parser and structured `StyleWarning`) merged.
- Sub-spec #2 (`--strict-style`, active wiring phase, CLI/frontmatter
  integration) merged.
- Sub-specs #3 and #4 (component width/max-width conflict precedent) merged.
- Sub-spec #5 (component color/bg-color storage and rendering) merged.

## Design Decisions

1. **`style.hr` is canonical; top-level `hr` is a temporary alias.**
   The existing top-level frontmatter block:

   ```yaml
   hr:
     style: waves
   ```

   must continue to work for one release cycle, but the parser should emit a
   structured deprecation warning whose canonical replacement is `style.hr`.
   If both `style.hr` and top-level `hr` are present, `style.hr` wins
   field-by-field and top-level `hr` only fills missing values.

2. **Inline `kind` is canonical; inline `style` is a temporary alias.**
   `RuleProcessor` should read `kind` first. If `kind` is absent and `style`
   is present, it should use the legacy value and retain enough source
   information for the render path / strict-style path to emit a structured
   `Deprecated` warning. If both are present, `kind` wins and the legacy
   `style` key still emits a deprecation warning because the document contains
   deprecated syntax.

3. **Precedence is inline > `style.hr` > top-level `hr` alias > component
   default.**
   Per-rule attributes are the most specific. `style.hr` is the canonical
   page-wide default. Top-level `hr` exists only for compatibility. Missing
   values fall through to `HorizontalRule::new()` defaults.

4. **Use typed enums at the schema boundary.**
   Define `HrKind` and `HrWeight` in `darkmatter::style::schema::hr` and map
   them to `RuleStyle` / `RuleWeight` at apply time. `HrKind` variants must be
   exactly:
   `Dashes`, `Dots`, `Waves`, `LineStar`, `LineCircle`, `InsetLine`,
   `CurtainRod`. `HrWeight` variants must be `Thin`, `Medium`, `Thick`.

5. **HR gets a specialized schema shape instead of flattening
   `CommonStyle`.**
   `CommonStyle.alignment` lowers to `renderable::layout::Alignment`, which
   cannot represent the HR-specific `full` value. For `HrStyle`, repeat the
   common fields that do apply (`width`, `max-width`, `color`, `bg-color`) and
   define `alignment: Option<HrAlignment>` directly. `style.hr.alignment`
   accepts `full | left | center | right`; `centered` is accepted as a
   deprecated alias for one release because existing inline HR syntax uses it.

6. **`weight` is intentionally added to the `style.hr` schema.**
   The parent `style.md` originally listed only `kind` as HR-specific, but the
   live top-level `hr:` contract already supports `weight`. Omitting it would
   make this migration a silent feature loss. Adding `style.hr.weight` is the
   least surprising compatibility path; update the parent style docs and
   schema descriptor accordingly.

7. **`width` and `max-width` conflict.**
   `HorizontalRule` currently exposes a single width string, and the earlier
   reviewed component specs reject simultaneous `width` and `max-width`
   because the current layout storage cannot represent both. Apply the same
   rule here: if `style.hr.width` and `style.hr.max-width` are both set in the
   canonical/legacy merged defaults, return `StyleApplyError` before
   rendering. Inline attributes only support `width` in this sub-spec.

8. **`style.hr.color` and `style.hr.bg-color` use sub-spec #5 color storage.**
   Add `PageComponent::Hr` and include it in color/bg-color handling introduced
   by sub-spec #5. HR foreground color maps to the rule stroke/fill color.
   Background color applies to the HR component's bounding line/box through the
   same wrapper mechanism used for other `PageComponent` variants.

9. **`--strict-style` rejects deprecated HR syntax.**
   Strict mode already promotes `StyleWarningKind::Deprecated` to errors.
   This sub-spec must route both top-level `hr:` alias usage and inline
   `--- { style: ... }` usage through that warning channel so
   `md --strict-style` exits non-zero before producing final output.

10. **Warnings use canonical paths.**
    Use `hr` -> `style.hr` for top-level alias warnings,
    `hr.inline.style` -> `hr.inline.kind` for inline attribute warnings, and
    `style.hr.alignment: centered` -> `style.hr.alignment: center` for the
    deprecated frontmatter alignment spelling.

## Public API

```rust
// darkmatter::style::schema::hr - modified

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HrKind {
    Dashes,
    Dots,
    Waves,
    LineStar,
    LineCircle,
    InsetLine,
    CurtainRod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HrWeight {
    Thin,
    Medium,
    Thick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HrAlignment {
    Full,
    Left,
    Center,
    Right,
}

pub struct HrStyle {
    #[serde(deserialize_with = "deserialize_optional_length")]
    pub width: Option<Length>,
    #[serde(
        deserialize_with = "deserialize_optional_length",
        alias = "max_width"
    )]
    pub max_width: Option<Length>,
    #[serde(deserialize_with = "deserialize_optional_color")]
    pub color: Option<StyleColor>,
    #[serde(
        deserialize_with = "deserialize_optional_color",
        alias = "bg_color"
    )]
    pub bg_color: Option<StyleColor>,
    pub alignment: Option<HrAlignment>,
    pub kind: Option<HrKind>,
    pub weight: Option<HrWeight>,
}

// darkmatter::layout - modified after sub-spec #5

pub enum PageComponent {
    Images,
    BlockQuotes,
    Tables,
    CodeBlocks,
    Ul,
    Ol,
    Li,
    Hr,
    #[deprecated(note = "use PageComponent::{Ul, Ol, Li}")]
    Lists,
}

// darkmatter::style - extended

pub fn apply_hr_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    overrides: HrStyleOverrides,
) -> Result<DarkmatterPage, StyleApplyError>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HrStyleOverrides {
    pub alignment: bool,
    pub fill: bool,
    pub color: bool,
    pub bg_color: bool,
}
```

`HorizontalRuleAttrs` should also stop treating `style: Option<String>` as the
canonical field. Use a shape that can retain deprecation provenance, for
example:

```rust
pub struct HorizontalRuleAttrs {
    pub kind: Option<String>,
    pub legacy_style: Option<String>,
    pub alignment: Option<String>,
    pub weight: Option<String>,
    pub width: Option<String>,
    pub color: Option<String>,
}
```

The implementation may choose a more precise enum for inline source
provenance, but it must preserve enough information to warn when a deprecated
key was present.

## Implementation Notes

- Update `darkmatter/lib/src/style/descriptor.rs` to add `hr.weight` and to
  keep `hr.kind`, `hr.width`, `hr.max-width`, `hr.alignment`, `hr.color`, and
  `hr.bg-color` at `sub_spec: 6`.
- Update descriptor validation for `hr.alignment`: the existing generic
  `LeafType::Alignment` is not sufficient unless it is extended to accept
  `full` only for the HR path. Prefer a dedicated HR alignment leaf type or a
  path-specific validator so non-HR component alignments do not accidentally
  accept `full`.
- Advance `ACTIVE_STYLE_WIRING_SUB_SPEC` to `6` only after HR wiring is active.
- Replace `hr_defaults_from_frontmatter(md)` with a helper that consumes the
  parsed `StyleFrontmatter` plus a deprecated top-level `hr:` compatibility
  value. Do not keep a separate top-level-only path after this sub-spec.
- Keep scalar coercion for the top-level `hr:` alias for compatibility with
  current docs (`width: 50` should still behave like `width: "50"`), but the
  canonical `style.hr` path should follow the typed style parser's existing
  deserializers.
- Map `HrKind` to `RuleStyle`, `HrWeight` to `RuleWeight`, and `HrAlignment`
  to `RuleAlignment` in one shared helper used by both terminal and browser
  renderers.
- `style.hr.max-width` lowers to the same effective rule width channel as
  `style.hr.width`, but as a cap against the current page content width. If
  this cannot be represented without changing `HorizontalRule`, return a
  clear `StyleApplyError` instead of silently ignoring `max-width`.
- Update both terminal and browser paths; the current shared
  `build_rule_with_defaults` helper is the right consolidation point.
- Avoid adding CLI flags for HR in this sub-spec. Existing global color/fill
  flags from prior specs may claim HR fields through `HrStyleOverrides` if
  those flags were designed to broadcast to every `PageComponent`.

## Reader Note

The reviewed spec intentionally changes the draft in three places:

- It includes top-level `hr:` in the migration. The implementation currently
  reads that block, so omitting it would leave the biggest compatibility path
  unspecified.
- It adds `style.hr.weight`. The existing HR contract already has `weight`;
  dropping it would be an accidental behavior regression, not a simplification.
- It requires deprecation provenance for inline `style`. Without that, strict
  mode cannot reliably reject `--- { style: waves }` because the current attrs
  type loses the fact that a deprecated key was used.

## Tests

1. **Frontmatter HR kind** - `style.hr.kind: waves` plus a document with
   bare `---` renders every HR as waves.
2. **Top-level `hr` alias** - top-level `hr: { style: waves }` still renders
   as waves and emits `Deprecated { replacement: "style.hr" }`.
3. **`style.hr` beats top-level `hr`** - top-level `hr.style: dots` plus
   `style.hr.kind: waves` renders waves.
4. **Inline HR kind (new syntax)** - `--- { kind: waves }` renders waves with
   no warning.
5. **Inline HR kind (legacy syntax)** - `--- { style: waves }` renders waves
   and emits one `Deprecated { replacement: "kind" }` warning.
6. **Inline canonical beats legacy** - `--- { kind: dots, style: waves }`
   renders dots and still emits the legacy-key deprecation warning.
7. **Inline overrides frontmatter** - frontmatter sets `style.hr.kind: waves`,
   inline says `kind: dots`, and the rendered HR uses dots.
8. **HR weight from frontmatter** - `style.hr.weight: thick` maps to
   `RuleWeight::Thick` on both terminal and browser paths.
9. **HR width conflict** - `style.hr.width` plus `style.hr.max-width` returns
   `StyleApplyError` before rendering.
10. **HR color** - `style.hr.color: red-500` renders the HR stroke/fill with
    the red foreground color on terminal and browser targets.
11. **HR bg-color** - `style.hr.bg-color: black` wraps the HR component with a
    black background using the sub-spec #5 component background mechanism.
12. **`--strict-style` rejects top-level alias** - strict mode exits non-zero
    on a document that uses top-level `hr:`.
13. **`--strict-style` rejects inline legacy syntax** - strict mode exits
    non-zero on `--- { style: waves }`.
14. **Active wiring warnings** - every `style.hr.*` key wired here emits no
    `KnownButInactive { sub_spec: 6 }`.

## Acceptance Criteria

- `style.hr.{kind,width,max-width,alignment,weight,color,bg-color}` are parsed
  and wired for terminal and browser rendering.
- Top-level `hr:` continues to work for one release cycle as a deprecated
  alias and is documented as deprecated.
- Inline `style` continues to work for one release cycle as a deprecated alias
  for `kind` and is documented as deprecated.
- `HrStyle.kind` is a typed enum, not `Option<String>`.
- `HrStyle.weight` exists and maps to `RuleWeight`.
- `PageComponent::Hr` exists and is honored by sub-spec #5 color/bg-color
  handling.
- `KnownButInactive { sub_spec: 6 }` warnings no longer fire for HR keys wired
  here.
- `darkmatter/docs/rendering/hr.md` and `darkmatter/docs/rendering/style.md`
  are updated to document `style.hr` as canonical, top-level `hr` as
  deprecated, inline `kind` as canonical, and inline `style` as deprecated.
- All previous sub-spec tests pass.

## Risks

- **Warning plumbing scope.** Inline HR attributes are discovered inside the
  Markdown render path, while `--strict-style` currently starts from
  frontmatter parsing. Mitigation: preserve deprecation provenance in
  `HorizontalRuleAttrs` and add a render-preflight pass or renderer option
  that converts inline HR deprecations into `StyleWarning` values before final
  output is emitted.
- **Compatibility noise.** Documents using existing top-level `hr:` or inline
  `style` will start warning. Mitigation: keep aliases for one release cycle,
  document the migration, and mention it in CHANGELOG.
- **Width semantics.** `HorizontalRule` has one width channel while the style
  schema has `width` and `max-width`. Mitigation: reject simultaneous use and
  test that `max-width` either lowers to an actual cap or fails clearly if the
  current component cannot support it.
- **Enum drift.** `HrKind` must stay aligned with biscuit-terminal
  `RuleStyle`. Mitigation: mapping tests cover every variant and fail when
  `RuleStyle` adds a new variant.

## Open Questions

None. This review resolves the HR migration design choices needed for planning
and implementation.

## Out-of-Spec

After sub-spec #6, HR styling is fully migrated into `style.hr`. Sub-spec #7
picks up the last bespoke knobs (`page.stylesheet`, `page.meta`, `page.code`,
`hyperlinks.local-style`, `images.local-style`).

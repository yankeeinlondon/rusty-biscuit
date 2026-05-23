---
status: draft
date: 2026-05-23
owner: ken
parent: docs/rendering/style.md
sub-spec: 6-of-7
depends-on: spec.md, spec-2.md..spec-5.md
---

# `style:` Frontmatter — Sub-Spec #6: HR Migration

## Problem

HRs (horizontal rules) have a legacy per-block style mechanism living in
`darkmatter/lib/src/markdown/block/hr_builder.rs:117`. The `style:`
frontmatter contract reserves `style.hr.kind` (and the rest of
`CommonStyle`) for HR styling, but until this sub-spec lands the legacy
mechanism is the only thing that affects HR rendering.

`docs/rendering/style.md` explicitly flags this:

> **IMPORTANT:** currently the `hr` functionality has implemented their
> bespoke styles directly to the top-level `hr` property and that needs
> to be moved here as `style.hr`

This sub-spec executes that migration plus the rename `hr.style` →
`hr.kind` (so `--- { style: waves }` becomes `--- { kind: waves }`).

## Goals

- Move all HR styling logic from `hr_builder.rs` to consume
  `style.hr.*` from the frontmatter (resolved at parse/render time).
- Rename the inline HR-attribute key `style` → `kind` so the inline form
  matches the schema:
  - Old: `--- { style: waves, alignment: centered, color: red }`
  - New: `--- { kind: waves, alignment: centered, color: red }`
- Accept the old key spelling as a deprecated alias for one release
  cycle; emit a `Deprecated` warning.
- Wire `style.hr.kind` (plus `width`, `max-width`, `alignment`, `color`,
  `bg-color`) to the renderer.
- Replace `Option<String>` `kind` field in `HrStyle` with a typed enum
  matching the HR-kind taxonomy (`Line`, `Waves`, `Dots`, `Dashes`,
  `LineStar`, etc. — pull from `hr_builder.rs`).
- Suppress `KnownButInactive { sub_spec: 6 }` warnings for wired keys.

## Non-Goals

- No new HR kinds beyond what `hr_builder.rs:117` already supports.
- No SVG / image HR support (separate feature).

## Dependencies

- Sub-specs #1–#5 merged.

## Decisions to Settle (Brainstorm Inputs)

1. **`hr.kind` typed enum location.** Define `HrKind` enum in
   `darkmatter::style::schema::hr` (with `Deserialize` and string
   matching). Replace the `Option<String>` field on `HrStyle` with
   `Option<HrKind>`. Variants: `Line`, `Waves`, `Dots`, `Dashes`,
   `LineStar`, `LineDash`, etc. — read `hr_builder.rs` for the
   authoritative list.

2. **Inline-attribute migration.** Currently `hr_builder.rs:141` reads
   `"style"` as the HR-kind attribute. Two changes:
   - Read `"kind"` first (canonical).
   - Read `"style"` if `kind` is absent; emit a `Deprecated` warning.

3. **Precedence between frontmatter and inline attributes.** Three
   sources can specify HR styling for a given HR:
   - Frontmatter `style.hr.kind: waves` (page-wide default).
   - Inline `--- { kind: waves }` (per-block).
   - Old inline `--- { style: waves }` (deprecated).

   Recommended: inline > frontmatter > default. The block-level
   attribute is the most specific.

4. **`style.hr.color` vs. inline `color`.** Sub-spec #5 wires component
   colors via `with_component_color(PageComponent::Hr, ...)`. We may
   need to add a new `PageComponent::Hr` variant (currently absent).
   Confirm.

5. **Per-block frontmatter precedence within a multi-HR document.** If
   the document has multiple HRs and `style.hr.kind: waves` is set, does
   every HR render as waves? Yes — frontmatter is a default that every
   HR inherits unless overridden inline.

6. **Migration warning loudness.** Old `--- { style: waves }` syntax
   continues to work but warns. Decide: should `--strict-style`
   (introduced in sub-spec #2) treat this as a hard error? Yes, per the
   strict-mode contract.

## Public API (Sketch)

```rust
// darkmatter::style::schema::hr — modified

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HrKind {
    Line,
    LineDash,
    LineStar,
    Waves,
    Dots,
    Dashes,
    // + any other variants in hr_builder.rs
}

pub struct HrStyle {
    #[serde(flatten)]
    pub common: CommonStyle,
    pub kind: Option<HrKind>,  // was Option<String>
}

// darkmatter::layout — modified

pub enum PageComponent {
    Images,
    BlockQuotes,
    Tables,
    CodeBlocks,
    Ul, Ol, Li,
    Hr,                // ← new variant
}
```

## Tests

1. **Frontmatter HR kind** — `style.hr.kind: waves` + a document with
   `---` → renders as waves on every HR.
2. **Inline HR kind (new syntax)** — `--- { kind: waves }` → renders as
   waves, no warning.
3. **Inline HR kind (legacy syntax)** — `--- { style: waves }` →
   renders as waves, emits one `Deprecated` warning.
4. **Inline overrides frontmatter** — frontmatter sets `kind: waves`,
   inline says `kind: dots` → renders as dots.
5. **HR color** — `style.hr.color: red-500` → HR renders with red SGR.
6. **`--strict-style` rejects legacy `style:` syntax** — strict mode
   exits non-zero on `--- { style: waves }`.

## Acceptance Criteria

- All HR-related logic in `hr_builder.rs` reads through the new
  `style.hr.*` AST instead of its inline `style` attribute.
- The string `"style"` attribute key continues to work with a
  `Deprecated` warning for one release.
- `HrKind` is a typed enum (no more `Option<String>`).
- `PageComponent::Hr` exists and is honored by sub-spec #5 color/bg-color.
- Documentation in `darkmatter/docs/rendering/hr.md` and
  `docs/rendering/style.md` updated to reflect the migration.
- All previous sub-spec tests pass.

## Risks

- **Breaking change for existing users.** Any doc using
  `--- { style: waves }` will start emitting warnings. Acceptable
  because the alias still parses, but call it out in CHANGELOG.
- **Multi-source precedence ambiguity.** Three sources can specify HR
  kind. Settle the precedence rule in advance and document it.
- **`hr_builder.rs` refactor scope.** Migrating from inline reads to
  frontmatter-driven defaults touches the HR rendering path. Audit for
  inadvertent regressions.

## Open Questions

1. Authoritative list of `HrKind` variants — read `hr_builder.rs:117` to
   enumerate.
2. Inline > frontmatter > default precedence — confirm.
3. CHANGELOG / breaking-change notice strategy.

## Out-of-Spec

After sub-spec #6, HR is fully migrated. Sub-spec #7 picks up the last
bespoke knobs (`page.stylesheet`, `page.meta`, `page.code`,
`hyperlinks.local-style`, `images.local-style`).

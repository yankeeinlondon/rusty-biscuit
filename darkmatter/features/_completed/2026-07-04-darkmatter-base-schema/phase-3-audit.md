# Phase 3 Audit & Open-Question Resolutions

Records the frontmatter property audit and the four open-question decisions
made while freezing `darkmatter/docs/schemas/darkmatter.yaml` (Phase 3 of the
base-schema plan).

## Baseline Property Audit

Each surface that reads Darkmatter frontmatter was compared against the
baseline property list. Findings per key:

| Key | Surfaces that read/interpret it | Decision |
|-----|---------------------------------|----------|
| `$schema` | compose (schema validation, expression functions) | kept |
| `title` | compose (`markdown_title()` expression function) | kept |
| `description` | compose (schema-validation error context) | kept |
| `tags` | none (user data only; appears in tests) | kept — conventional document property the spec lists explicitly |
| `draft` | none (user data only; hash-ignore example) | kept — conventional document property the spec lists explicitly |
| `metadata` | none (user data; Darkmatter preserves it opaquely) | kept — Darkmatter preserves it and the spec lists it |
| `last_updated` | hash (read + mutated on content change) | kept |
| `hash` | hash (read + mutated; shorthand or longhand form) | kept (modeled as a union matching the runtime `StoredHash` shape) |
| `style` | style::parse (authoritative runtime parser) | kept as broad `object` (see Open Question 2) |
| `change` | **none** — `FrontmatterChange` is a delta diff-output struct, not a frontmatter key; the delta surface iterates all keys generically | kept as `any` (see Open Question 3) |
| `replace` | compose (effective state, replacement engine; object map) | kept |
| `ctx` | compose (effective state, expression namespace; object) | kept (nested object with `generated; required` leaves) |
| `prologue` | compose (transclusion; string or string[]) | kept (union) |
| `epilogue` | compose (transclusion; string or string[]) | kept (union) |
| `ignore_invalid` | compose + transform (bool) | kept |
| `interpolate_code_blocks` | compose inline interpolation (bool) | kept |

### Properties considered but deliberately excluded

- **Root-level `hr`** — read as a deprecated alias by both `style::parse`
  (`merge_deprecated_top_level_hr`) and `render_tree::entrypoints`
  (`hr_defaults_from_frontmatter`). Excluded per Non-Goal 6; the runtime
  compatibility paths are removed in Phase 5 and `style.hr.*` is the only
  horizontal-rule surface.

### Conclusion

The baseline property list is complete and correct for v1. No Darkmatter-owned
frontmatter key is missing, and no listed key is a non-Darkmatter contract.

## Open-Question Resolutions

### Open Question 2 — `style` shape

**Decision: `style` remains a broad `object` in v1.**

Rationale:

1. **Non-Goal 2** is explicit: "Do not fully model every nested runtime DSL in
   v1. Runtime parsers remain authoritative for rich nested surfaces such as
   `style`."
2. The spec Source Of Truth endorses broad `object` "when another Darkmatter
   parser is the more precise authority, when the shape is intentionally open,
   or when the shape is still moving" — `style` is all three.
3. **Drift found between the plan/skill and the code.** The plan and the
   darkmatter skill both reference `ACTIVE_STYLE_WIRING_SUB_SPEC = 9`, but the
   actual constant in `style/parse.rs:22` is `8`. The descriptor
   (`style/descriptor.rs`) does contain `sub_spec: 9` leaves
   (`margin`/`padding`/`border`/`emphasis`/`word-wrap` on every component plus
   the full `code-block` bucket), but the runtime flags any leaf with
   `sub_spec > ACTIVE_STYLE_WIRING_SUB_SPEC` as `KnownButInactive`
   (`style/parse.rs:273`). Fully modeling `style` in the baseline would
   therefore advertise keys the runtime considers inactive.
4. The draft's own comment conceded "Runtime validation and application remain
   authoritative in `darkmatter::style`."

A future phase may add partial inline-object modeling once the runtime wiring
constant and the descriptor are reconciled and the shape stabilizes.

### Open Question 3 — `change` validation shape

**Decision: `change` stays `any` in v1.**

The audit confirmed no Darkmatter surface reads a `change:` frontmatter key;
`FrontmatterChange` is a diff-output struct produced by the delta surface, not
a consumed property. `any` is retained because the spec lists `change` in the
baseline scope and the plan's default is to keep it open until a public change
contract is finalized (Non-Goal 2).

### Open Question 4 — generated property tables in docs

**Decision: transclusion only for v1.**

The docs page (`darkmatter/docs/schemas/darkmatter-schema.md`, landed in Phase 6)
transcludes the schema YAML verbatim and does not emit separate generated
property tables. This keeps the documentation and the validation source from
drifting — there is exactly one source of truth.

### Open Question 1

Not in Phase 3 scope (resolved in Phase 7 — CLI integration).

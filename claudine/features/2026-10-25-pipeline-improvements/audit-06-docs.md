---
kind: implementation-log
---

# Documentation audit: 2026-10-16-core-schema-types

Updated **4 existing canonical documents**; created **0 new document paths**.
One updated document, `schema-types.md`, was previously an empty placeholder,
so this audit supplies its missing content. The completed spec now references
11 canonical documents, all under a package area's `docs/` folder and all
with `kind: canonical-documentation`.

| Canonical document | Requirements covered | Change in this step |
| --- | --- | --- |
| `darkmatter/docs/topics/schemas/schema-types.md` | Choosing core types; unknown/any, native null, whole-value presence versus list items, root-union exclusivity, alphanumeric, passive shell, editor/CLI catalog, subtype use | Filled the existing zero-byte page with a practical introduction, type-choice table, presence table, and complete schema examples. It links to the detailed reference rather than duplicating the full grammar. |
| `darkmatter/docs/topics/schemas/definition.md` | Complete lowering, presence, null positions/defaults, options/canonicalization/merging, coercion, subtype APIs, trigger migration, shell action schemas, catalog | Linked the new guide; corrected the obsolete claim that unknown arguments receive no static checking; added concrete presence-gate guidance and malformed-value behavior; clarified that ordinary constraints are ignored inside structured subtype identities without widening object shapes. Existing detailed sections remain the canonical reference. |
| `darkmatter/docs/topics/schemas/authoring-schemas.md` | Required nullable properties and phase behavior | Reviewed unchanged; already current and canonical. |
| `darkmatter/docs/topics/schemas/local-type-references.md` | Built-in keyword collisions, inherited constraints and presence | Reviewed unchanged; already current and canonical. |
| `darkmatter/docs/topics/schemas/dmls-schema-support.md` | Core-type completion/hover, option/default diagnostics, native null source locations | Reviewed unchanged; already current and canonical. |
| `darkmatter/docs/inline/schema-validation.md` | CLI validation, null-admitting properties, raw JSON Schema distinction | Reviewed unchanged; already current and canonical. |
| `darkmatter/docs/inline/doc-list.md` | The renamed unrestricted type in planned filter/enumeration examples | Set canonical kind and added an explicit planned marker. Existing `unknown`/`defines` guidance is correct. The page remains future behavior and does not imply that its directive is implemented. |
| `darkmatter/dmls/docs/autocomplete.md` | Built-in keyword/alias and alphanumeric-option completion | Reviewed unchanged; already current and canonical. |
| `darkmatter/dmls/docs/hover.md` | Canonical names/options, effective character sets, built-in collision navigation | Reviewed unchanged; already current and canonical. |
| `claudine/docs/topics/composition.md` | Nullable presence, interactive input for the new types | Reviewed unchanged; already current and canonical. |
| `claudine/docs/topics/state-management/side-effects.md` | Null accepted by set_frontmatter; corrected two-argument object merge shape | Set canonical kind; added explicit-null storage and object-merge examples, distinguishing null from deletion and rejecting scalar/list merge input. |

## Integrated behavior and skill drift

Later type-aware evaluation classifies a fully unknown function argument as
category 4, which warns only when DMLS `expressions.strict` is enabled; normal
composition does not reject it as proven incompatible. The definition page
and `.claude/skills/darkmatter/schema.md` now say that precisely while retaining
runtime value checks. This is a skill update, not an extra canonical-document
count. The relation's current extension to recursive types is preserved; no
older subtype restriction was reinstated.

## Validation

Compared behavior with the current `simplified/alphanumeric.rs`,
`simplified/subtype.rs`, schema descriptor catalog, `call_check.rs`,
`diagnostics.rs`, and `effects/verbs.rs`, as well as existing current topic
pages. Checked all 11 reference paths and kind values, every new guide anchor,
and the completed specification's unchanged body against HEAD. No canonical
document gained a feature/fix reference or dated spec name. `git diff --check`
passes. This documentation slice ran no tests or formatter; the concurrent
test agent owns executable verification.

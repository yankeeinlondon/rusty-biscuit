---
ready: false
implemented: true
agent: codex/default
created: 2026-07-05T18:52:05
---

# Review 4: Darkmatter Base Frontmatter Schema

## Findings

### High: `generated` is not exposed through the completion metadata API

The spec requires generated properties to surface in editor/completion metadata: "LSP/editor tooling should expose generated properties for completion, hover, and diagnostics" (`darkmatter/features/2026-07-04-darkmatter-base-schema/spec.md:167`) and the testing checklist explicitly requires generated properties to retain their semantics in "runtime/effective schemas and LSP/completion metadata" (`darkmatter/features/2026-07-04-darkmatter-base-schema/spec.md:339`). The implementation verifies the JSON Schema side: `generated; required` suppresses static requiredness, emits `x-darkmatter-generated: true`, and preserves the non-nullable typed fragment (`darkmatter/lib/src/markdown/schemas/simplified/convert.rs:1727`).

The public completion surface does not carry that bit. `CompletionSuggestion` exposes only `property`, `is_array`, `description`, and `kind` (`darkmatter/lib/src/markdown/schemas/completion.rs:43`), and `suggestion_from_def` copies only those fields from the selected atom (`darkmatter/lib/src/markdown/schemas/completion.rs:182`). The completion tests cover files, enums, hints, arrays, descriptions, and unions, but there is no test proving a `file(generated)`, `enum(...; generated)`, or other completable generated atom is reported as host-supplied (`darkmatter/lib/src/markdown/schemas/completion.rs:280`). A downstream LSP/completion consumer using the documented API in `schema-definition.md` has no way to distinguish a user-authored field from a runtime-supplied/generated field, even though the converted schema contains that information.

This should be fixed by adding generated ownership metadata to `CompletionSuggestion` (for example, `pub generated: bool`) and deriving it from `Constraint::Generated` across single properties, property unions, and root unions. Add Level 1 tests for generated completable atoms and generated union arms. If the intended contract is that completion callers must inspect `EffectiveSchema::json_schema` directly instead, the docs and spec need to say that; the current public API and docs imply `CompletionSuggestion` is the completion metadata surface.

Verification level: Level 1 is the appropriate level. This is in-process schema metadata behavior with no terminal rendering, real-terminal capture, or OS input requirement. Current strongest coverage is Level 1 for JSON Schema annotation only; completion metadata itself is unverified and incomplete.

## Test-Level Assessment

- Base schema parsing, conversion, known valid/invalid values, unknown top-level keys, document `$schema` precedence, and closed generated `ctx` behavior have appropriate Level 1 coverage.
- Nested YAML mapping object syntax and property-level sequence unions with mapping arms have appropriate Level 1 parser/conversion/validation coverage.
- `generated` parsing, static-required suppression, JSON Schema annotation emission, and present-value type/nullability checks have appropriate Level 1 coverage.
- `generated` completion metadata does not have the required Level 1 coverage, and the API currently cannot expose the required metadata.
- `md compose` default baseline injection, `--no-baseline-schema`, `DARKMATTER_NO_BASELINE_SCHEMA`, explicit baseline replacement, and custom `ctx.*` rejection have appropriate Level 1 CLI coverage.
- Existing `md schema about` real-terminal rendering coverage remains Level 2, which is appropriate for terminal report rendering.
- No Level 3 requirements apply; this feature has no OS keyboard or mouse input behavior.

## Summary

The Review 3 documentation gap appears fixed: the public schema docs now describe default `md compose` base-schema injection, opt-outs, explicit replacement, and the intentional divergence from `md schema validate`.

The feature is still not production-ready because the `generated` constraint is only exposed in converted JSON Schema, not through the public completion metadata API that the spec calls out for LSP/editor consumers.

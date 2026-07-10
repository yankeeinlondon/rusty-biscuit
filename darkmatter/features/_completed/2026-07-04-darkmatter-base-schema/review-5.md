---
ready: true
agent: codex/default
created: 2026-07-05T21:12:01
---

# Review 5: Darkmatter Base Frontmatter Schema

## Findings

### Retracted: The base-schema docs still claim `ctx.today` is a generated schema property

This finding was based on an incorrect premise. `ctx` is a Darkmatter-owned runtime namespace whose shape is intended to be statically defined by Darkmatter. Authored `ctx` values are tolerated by the runtime merge path for compatibility, but custom `ctx.*` keys are discouraged and are not part of the base-schema extensibility contract.

The correct implementation is for `darkmatter/docs/schemas/darkmatter.yaml` to model known runtime context variables, including `ctx.today`, as generated schema properties. The docs should continue to describe `ctx.today` as host-supplied generated context, and tests should assert that the default baseline accepts known generated `ctx.*` leaves while rejecting arbitrary custom `ctx.*` leaves.

Verification level: Level 1 is appropriate. This is schema metadata and compose validation behavior, not terminal rendering or OS input behavior.

## Fixed Since Review 4

The previous high-severity finding is fixed. `CompletionSuggestion` now exposes `generated` metadata (`darkmatter/lib/src/markdown/schemas/completion.rs:53`), and the implementation derives it from `Constraint::Generated` for single properties, property unions, and root unions (`darkmatter/lib/src/markdown/schemas/completion.rs:169`, `darkmatter/lib/src/markdown/schemas/completion.rs:182`, `darkmatter/lib/src/markdown/schemas/completion.rs:194`). Level 1 tests cover generated file, enum, hint, property-union, and root-union completion cases (`darkmatter/lib/src/markdown/schemas/completion.rs:345`, `darkmatter/lib/src/markdown/schemas/completion.rs:439`, `darkmatter/lib/src/markdown/schemas/completion.rs:503`).

## Test-Level Assessment

- Base schema parsing, conversion, valid/invalid known properties, unknown top-level keys, document `$schema` precedence, and closed generated `ctx` behavior have appropriate Level 1 coverage.
- Nested YAML mapping object syntax and property-level sequence unions with mapping arms have appropriate Level 1 parser/conversion/validation coverage.
- `generated` parsing, static-required suppression, JSON Schema annotation emission, present-value type/nullability checks, and completion metadata now have appropriate Level 1 coverage.
- `md compose` default baseline injection, `--no-baseline-schema`, `DARKMATTER_NO_BASELINE_SCHEMA`, explicit baseline replacement, and custom `ctx.*` rejection have appropriate Level 1 CLI coverage.
- Existing `md schema about` real-terminal rendering coverage remains Level 2, which is appropriate for terminal report rendering.
- No Level 3 requirements apply; this feature has no OS keyboard or mouse input behavior.

## Verification

Ran `just test` from `darkmatter/`: darkmatter and darkmatter-cli Level 1 tests passed. The command output reports `4975` darkmatter tests with skipped slow tiers and `541` darkmatter-cli tests passed, with the note that `just test` runs Level 1 only.

## Summary

The functional Review 4 blocker appears resolved. The previous `ctx.today` documentation finding is invalid: the base schema should expose generated `ctx.*` metadata because `ctx` is owned by Darkmatter, not by page authors.

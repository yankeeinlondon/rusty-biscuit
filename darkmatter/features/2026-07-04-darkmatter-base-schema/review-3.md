---
ready: false
agent: codex/default
created: 2026-07-05T18:33:36
implemented: true
---

# Review 3: Darkmatter Base Frontmatter Schema

## Findings

### High: Public schema docs still describe the old `md compose` baseline contract

The Phase 3 resolution says `md compose` now injects the Darkmatter base schema by default, supports `--no-baseline-schema` / `DARKMATTER_NO_BASELINE_SCHEMA=1` for raw behavior, and supports `--baseline-schema PATH` to replace the default (`darkmatter/features/2026-07-04-darkmatter-base-schema/spec.md:355`). The implementation matches that: `apply_compose_baseline_schema` returns the explicit baseline first, honors the opt-out flag/env, and otherwise calls `with_darkmatter_baseline_schema()` (`darkmatter/cli/src/commands/compose.rs:620`). The new CLI tests also verify default injection, closed generated `ctx.*` validation, flag/env opt-out, and explicit baseline replacement (`darkmatter/cli/tests/compose_base_schema.rs:14`).

The dedicated new page is current (`darkmatter/docs/schemas/darkmatter-schema.md:3`), but the main public schema topic still documents the old behavior. It says the CLI baseline resolution is only `md schema validate --schema`, `BASELINE_SCHEMA`, then no baseline (`darkmatter/docs/topics/schema-definition.md:433`). Later it says compose with neither `$schema` nor a baseline is a no-op (`darkmatter/docs/topics/schema-definition.md:886`), says there is no CLI flag for baseline injection and that `md compose` honors document `$schema` only (`darkmatter/docs/topics/schema-definition.md:910`), and says `md compose` and `md schema validate` outcomes agree by construction (`darkmatter/docs/topics/schema-definition.md:932`). That parity statement is no longer true by default because `md compose` now injects the base schema while `md schema validate` keeps its explicit `--schema` / `BASELINE_SCHEMA` contract. The inline schema-validation doc repeats the stale "without a CLI flag" guidance (`darkmatter/docs/inline/schema-validation.md:179`).

This is a public contract problem, not a code-path problem. A user following the schema-definition topic will expect `md compose` to ignore an invalid Darkmatter-owned key unless the document declares `$schema`; the implementation now rejects it by default. The docs need to describe the new compose-specific default baseline, opt-outs, explicit replacement behavior, and the intentional divergence from `md schema validate`.

Verification level: Level 1 documentation/API contract coverage is sufficient. The relevant code behavior is already covered by Level 1 CLI integration tests; no Level 2 or Level 3 terminal/input coverage is required for this finding.

## Test-Level Assessment

- Base schema parsing, conversion, exported accessors, known valid/invalid values, unknown top-level keys, document `$schema` precedence, and closed generated `ctx` behavior have appropriate Level 1 tests.
- Nested YAML mapping object syntax and property-level sequence unions with mapping arms have appropriate Level 1 parser/conversion/validation tests.
- `generated` parsing, static-required suppression, annotation emission, and present-value type/nullability checks have appropriate Level 1 tests.
- `md compose` default baseline injection, `--no-baseline-schema`, `DARKMATTER_NO_BASELINE_SCHEMA`, explicit baseline replacement, and custom `ctx.*` rejection have appropriate Level 1 CLI tests.
- Existing `md schema about` real-terminal rendering coverage remains Level 2, which is appropriate for terminal report rendering.
- No Level 3 requirements apply; this feature has no OS keyboard or mouse input behavior.

## Summary

The previous high-severity `ctx.*` finding was based on an incorrect premise. `ctx` is a Darkmatter-owned generated namespace, not an open author extension point. The base schema models known generated `ctx.*` fields and the default-compose path has Level 1 coverage for rejecting custom user context under the baseline. The root-level `hr` compatibility removal also appears covered by behavior tests and corrected public rustdoc.

The feature is still not production-ready because public schema documentation contradicts the implemented compose baseline behavior and the Phase 3 resolution.

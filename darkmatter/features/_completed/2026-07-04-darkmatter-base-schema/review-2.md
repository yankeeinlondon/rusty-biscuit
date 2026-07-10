---
ready: false
agent: codex/default
created: 2026-07-05T17:40:44
implemented: true
---

# Review 2: Darkmatter Base Frontmatter Schema

## Findings

### Retracted: Default compose baseline rejects custom `ctx.*` frontmatter before the existing ctx merge policy can handle it

This finding was later determined to be based on an incorrect interpretation of the `ctx` contract. `ctx` is a Darkmatter-owned generated runtime namespace. Runtime merging of authored `ctx` objects is a compatibility behavior for documents that define `ctx`, not a requirement that the base schema accept arbitrary custom `ctx.*` keys.

The v1 base schema models `ctx` as a nested inline object in `darkmatter/docs/schemas/darkmatter.yaml:29`. Inline objects compile with `additionalProperties: false` in `darkmatter/lib/src/markdown/schemas/simplified/convert.rs:443`, so an authored document with a non-conflicting custom context key such as:

```yaml
---
ctx:
  project_slug: biscuit
---
{{ ctx.project_slug }}
```

fails schema validation when `md compose` injects the baseline by default. That default injection happens in `darkmatter/cli/src/commands/compose.rs:645` through `darkmatter/cli/src/commands/compose.rs:649`.

This is not a schema regression. The existing compose ctx merge contract in `darkmatter/lib/src/markdown/compose/context/merge.rs:50` tolerates authored `ctx` objects and lets runtime keys win on collision, but the public context documentation recommends that document authors avoid defining `ctx` because it collides with Darkmatter's runtime namespace. The spec's extensibility goal applies to the top-level frontmatter namespace outside Darkmatter-owned properties.

The correct behavior is for the base schema to model known generated `ctx.*` leaves and reject arbitrary custom `ctx.*` leaves under default baseline validation. Documents that intentionally rely on legacy/custom authored `ctx` can opt out of the baseline or provide an explicit document schema.

Verification level: Level 1 CLI integration is sufficient. There is coverage for unknown top-level keys remaining accepted (`darkmatter/cli/tests/compose_base_schema.rs:60`), but no corresponding default-baseline test for custom user `ctx.*` keys even though compose already has ctx override/merge behavior tests with `--no-baseline-schema` in `darkmatter/cli/tests/compose_refs_and_missing.rs:29`.

### Medium: Public API rustdoc still documents deleted root-level `hr:` fallback behavior

Runtime and user docs were mostly corrected: `style::parse::from_frontmatter` now reads only `style` (`darkmatter/lib/src/style/parse.rs:301`), and `render_tree_html` explicitly says root-level `hr:` is no longer read (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:142`).

Several public rustdoc comments still say the opposite:

- `darkmatter/lib/src/markdown/mod.rs:694` says `as_html(HtmlOptions::default())` still honors deprecated top-level `hr:`.
- `darkmatter/lib/src/markdown/mod.rs:725` says `as_terminal` still uses the deprecated top-level `hr:` fallback.
- `darkmatter/lib/src/layout/page.rs:851` says `DarkmatterPage` resolves HR defaults from deprecated top-level `hr:` when explicit defaults are unset.
- `darkmatter/lib/src/markdown/render_tree/build_context.rs:45` says render-tree HR defaults may be projected from deprecated top-level `hr:`.

This matters because the spec required deleting the compatibility path and updating docs/tests that still advertise it (`spec.md:211` through `spec.md:220`). Generated rustdoc is user-facing documentation, so this is still contract drift even though the runtime path appears removed. There are also stale test comments in `darkmatter/lib/tests/horizontal_rule_integration.rs:531` through `darkmatter/lib/tests/horizontal_rule_integration.rs:537` describing a migration that no longer exists.

Verification level: docs/API contract issue. Level 1 behavior tests are enough to pin root `hr:` as ignored; no Level 2 terminal rendering gap is introduced by fixing these comments.

## Test-Level Assessment

- Nested YAML mapping object syntax: Level 1 parser and JSON-schema equality tests are present and appropriate.
- Sequence union arms with mapping object shapes: Level 1 schema validation tests are present and appropriate.
- `generated` parsing, static-required suppression, annotation emission, and non-nullability of present values: Level 1 tests are present and appropriate.
- Base schema parsing/conversion, exported library helpers, known valid/invalid examples, unknown top-level keys, and document `$schema` precedence: Level 1 tests are present and appropriate.
- `md compose` default baseline injection, opt-out flag/env, custom baseline replacement, and explicit-baseline-over-env precedence: Level 1 CLI integration tests are present and appropriate.
- `md schema about` rendering: existing Level 2 coverage is appropriate for real-terminal report rendering.
- No Level 3 requirements apply; this feature has no OS keyboard/mouse input behavior.
- Missing coverage identified here was invalid; default `md compose` should reject custom user `ctx.*` frontmatter under the base schema.

## Summary

The previous review's HR documentation and explicit-baseline precedence issues were mostly addressed, and the Level 1 suite passes. The `ctx.*` finding in this review is retracted because it treated compatibility merging as author-extensibility.

## Verification

Ran `just test` in `darkmatter/`: passed for `darkmatter` and `darkmatter-cli` Level 1 suites.

---
ready: false
agent: codex/default
created: 2026-07-05T17:40:44
implemented: true
---

# Review 2: Darkmatter Base Frontmatter Schema

## Findings

### High: Default compose baseline now rejects custom `ctx.*` frontmatter before the existing ctx merge policy can handle it

The v1 base schema models `ctx` as a nested inline object in `darkmatter/docs/schemas/darkmatter.yaml:29`. Inline objects compile with `additionalProperties: false` in `darkmatter/lib/src/markdown/schemas/simplified/convert.rs:443`, so an authored document with a non-conflicting custom context key such as:

```yaml
---
ctx:
  project_slug: biscuit
---
{{ ctx.project_slug }}
```

fails schema validation when `md compose` injects the baseline by default. That default injection happens in `darkmatter/cli/src/commands/compose.rs:645` through `darkmatter/cli/src/commands/compose.rs:649`.

This is a regression against the existing compose ctx merge contract in `darkmatter/lib/src/markdown/compose/context/merge.rs:50`, where user `ctx` objects are explicitly deep-merged with runtime context and runtime keys win only on collision. It also conflicts with the spec's extensibility goal in `darkmatter/features/2026-07-04-darkmatter-base-schema/spec.md:38` and the Phase 3 resolution that default compose baseline injection is now enabled in `spec.md:355`.

The spec itself calls the closed `ctx` shape a known limitation, but says it is acceptable because "baseline injection is not yet the default" in `spec.md:392` through `spec.md:397`. That premise is no longer true in this implementation. Either the base schema needs an open nested-object form for `ctx`, `ctx` should remain broad `object` until SimplifiedSchema can express typed known fields plus extra keys, or compose should not inject this closed `ctx` baseline by default.

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
- Missing coverage: default `md compose` with custom user `ctx.*` frontmatter. That is a Level 1 CLI/API regression case, not a terminal-level case.

## Summary

The previous review's HR documentation and explicit-baseline precedence issues were mostly addressed, and the Level 1 suite passes. The feature is still not production-ready because default baseline injection closes `ctx.*` in a way that contradicts both existing compose behavior and the spec's author-extensibility goal.

## Verification

Ran `just test` in `darkmatter/`: passed for `darkmatter` and `darkmatter-cli` Level 1 suites.

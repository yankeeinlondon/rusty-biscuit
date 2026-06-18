---
agent: codex
model: ""
ready: false
---

# Review: Inline Nested Object Schemas in SimplifiedSchema

## Findings

### Medium: schema documentation labels the validate command as `md schema about`

The spec requires `darkmatter/docs/topics/schema-definition.md` to document inline object syntax and point users to `md schema about` as the implementation-bound CLI reference. The document now includes that material, but the CLI section at `darkmatter/docs/topics/schema-definition.md:480` is titled `## CLI: md schema about` while the command block and option table immediately below document `md schema validate` (`darkmatter/docs/topics/schema-definition.md:482`). A real `md schema about` section appears later at `darkmatter/docs/topics/schema-definition.md:672`.

This makes the public topic page contradictory: readers scanning headings will land on the wrong command, and the duplicate `md schema about` headings make anchors/table-of-contents entries ambiguous. The code path for `md schema about` itself appears descriptor-backed and covered; this is a documentation completion issue.

Suggested fix: rename the first heading to `## CLI: md schema validate` or otherwise split the validate/about command sections so each heading matches its command block.

Verification level: Level 1 documentation/CLI tests are appropriate. Add or adjust a docs-oriented test if the docs pipeline has one; otherwise a focused review check is enough for this typo-level docs regression.

## Test Notes

No Level 2 or Level 3 coverage is required for this feature. The user-observable behavior is schema parsing, JSON Schema conversion, validation, compose-time frontmatter coercion, CLI report text, and documentation content. Level 1 unit/integration tests are the right verification level for the implemented behavior.

Focused checks run:

- `cargo test --color=never -p darkmatter schemas::simplified::grammar --lib` passed: 52 tests.
- `cargo test --color=never -p darkmatter schemas::coerce --lib` passed: 49 tests.
- `cargo test --color=never -p darkmatter schemas::about --lib` passed: 7 tests.
- `cargo test --color=never -p darkmatter-cli --test schema_about --test schema_validate` passed: 34 tests.

The two high-severity issues from review 1 appear addressed:

- Mixed inline objects now keep recognized coercion targets while leaving opaque siblings untouched.
- Inline object constraints before `[]` are now rejected by the grammar.

## Production Readiness

Not ready for production until the schema topic's CLI section heading is corrected. The implementation and Level 1 coverage for the core parser, conversion, coercion, validation, and CLI report surfaces look aligned with the spec after this iteration, but the required public documentation currently points readers at the wrong command section.

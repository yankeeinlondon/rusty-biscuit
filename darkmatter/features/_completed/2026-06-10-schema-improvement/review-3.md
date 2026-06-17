---
agent: codex
model: ""
ready: false
---

# Review: Inline Nested Object Schemas in SimplifiedSchema

## Findings

### High: default `md schema about` omits required sections

The spec says `md schema about` prints a human-readable report covering schema shapes, type vocabulary, constraints, inline object syntax, validation behavior, and operational notes (`darkmatter/features/2026-06-10-schema-improvement/spec.md:309`). The test strategy repeats that `md schema about` should include sections for inline object rules, validation behavior, and coercion (`darkmatter/features/2026-06-10-schema-improvement/spec.md:461`). The public topic page documents the same default command and sample output, including Inline Object Rules, Coercion Rules, and Validation Behavior (`darkmatter/docs/topics/schema-definition.md:672`).

The implementation only prints those descriptor-backed sections when the global verbose flag is set. `run_about` renders shapes, types, and constraints, then gates `inline_object_rule_descriptors()`, `coercion_rule_descriptors()`, and `validation_behavior_descriptors()` behind `verbose` (`darkmatter/cli/src/commands/schema/about.rs:41`). The current integration tests explicitly lock in the opposite of the spec by asserting that plain `md schema about` does **not** contain `Nested Objects`, `Compose-time Coercion`, or `Validation Notes` (`darkmatter/cli/tests/schema_about.rs:12`, `darkmatter/cli/tests/schema_about.rs:232`) and only checks those sections under `--verbose` (`darkmatter/cli/tests/schema_about.rs:244`).

This leaves the documented command incomplete for the primary discoverability use case. A user running the command exactly as specified does not see the inline object rules, the 32-level depth rule, description termination, `additionalProperties: false`, validation behavior, or coercion behavior unless they know to use an unrelated global verbosity flag.

Suggested fix: make plain `md schema about` render all spec-required descriptor sections. If a shorter report is desired, that should be a deliberate spec change with docs and tests updated to say the default is a summary and `--verbose schema about` is the complete reference.

Verification level: Level 1 is appropriate. Add or change the CLI integration test so `md schema about` itself asserts the inline object, validation behavior, and coercion sections are present. No Level 2 or Level 3 coverage is needed because this is CLI text/content behavior, not real terminal rendering or keyboard input behavior.

## Test Notes

No Level 2 or Level 3 coverage is required for this feature. The user-observable behavior is schema parsing, JSON Schema conversion, validation, compose-time frontmatter coercion, CLI report text, and documentation content. Level 1 unit/integration tests are the correct verification level.

Focused checks run:

- `cargo test --color=never -p darkmatter schemas::simplified::grammar --lib` passed: 52 tests.
- `cargo test --color=never -p darkmatter schemas::coerce --lib` passed: 49 tests.
- `cargo test --color=never -p darkmatter schemas::about --lib` passed: 7 tests.
- `cargo test --color=never -p darkmatter-cli --test schema_about --test schema_validate` passed: 37 tests.

The previous review items appear addressed:

- The schema topic's `md schema validate` / `md schema about` headings are no longer swapped.
- Mixed inline objects now keep recognized coercion targets while leaving opaque siblings untouched.
- Inline object constraints before `[]` are rejected by the grammar.

## Production Readiness

Not ready for production. The parser, conversion, validation, and compose-coercion surfaces are well aligned with the spec after the earlier fixes, but the default `md schema about` command still does not provide the complete descriptor-backed reference the feature promises.

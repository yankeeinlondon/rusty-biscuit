---
ready: false
agent: codex/default
created: 2026-06-24T08:37:43
implemented: true
---

# Review 1

## Verdict

Not production ready.

The implementation appears to thread property descriptions through the core data model, validation enrichment, CLI JSON output, CLI pretty text output, and the schema validation error block. The focused Level 1 tests I ran passed. The blocker is that the new terminal-styled description sub-lines are only verified at Level 1, while the specification and review instructions require real-terminal verification for user-visible styling.

## Findings

### High: New dimmed/italic terminal description lines lack Level 2 verification

The feature adds terminal styling for two user-observable render surfaces:

- `md schema validate` pretty output renders `problem.description` as a dimmed sub-line in `emit_problem_bullet` ([darkmatter/cli/src/commands/schema/validate.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/src/commands/schema/validate.rs:287)).
- `MarkdownError::SchemaValidationFailed` renders each problem description as `<i><dim>...</dim></i>` in the status block ([darkmatter/lib/src/markdown/errors/blocks.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/errors/blocks.rs:350)).

The new tests cover the text at Level 1:

- CLI pretty output asserts the description text is present, but not the rendered dim SGR or real-terminal layout ([darkmatter/cli/tests/schema_validate.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/tests/schema_validate.rs:540)).
- CLI JSON output asserts the new field, which is appropriately Level 1 ([darkmatter/cli/tests/schema_validate.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/tests/schema_validate.rs:581)).
- The status block unit test asserts the description text is present, but does not drive the new per-problem description through a real terminal ([darkmatter/lib/src/markdown/errors/blocks.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/errors/blocks.rs:599)).

There is an existing Level 2 schema-validation block test, but its fixture only exercises the older document-level `description:` line. It does not include a `-> ...` property description, so it cannot catch regressions in the new per-problem description path or prove the new sub-line renders dimmed/italic through WezTerm ([darkmatter/cli/tests/level2_errors.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/tests/level2_errors.rs:108)).

Required fix: add Level 2 coverage for the new user-visible terminal surfaces. At minimum:

- Add a `level2_...` test for `md compose` with a failing property that declares `-> The headline shown in listing pages`, and assert the per-problem description appears in `frame.plain` and has dim + italic SGR in `frame.raw`.
- Add Level 2 coverage for `md schema validate` pretty output, or extend an appropriate real-terminal CLI validate test, asserting the description sub-line appears and is dimmed in `frame.raw`.

Verification level classification:

- Description enrichment on `ValidationProblem`: Level 1 is sufficient and present.
- JSON output `description` field: Level 1 is sufficient and present.
- Pretty CLI dimmed description sub-line: requires Level 2; strongest coverage is Level 1.
- Compose/status-block dimmed-italic per-problem description sub-line: requires Level 2; strongest coverage for this new path is Level 1.

## Notes

I did not find a clear functionality gap in the resolver behavior from the staged implementation. The tests cover missing required, wrong type, pointer escaping, nested objects, arrays, nullable wrappers, root unions, union articulation, absent descriptions, whitespace-only suppression, message-equality suppression, and unknown-property suppression.

## Verification

Ran:

```bash
cargo nextest run -p darkmatter -p darkmatter-cli -E 'test(resolve_) + test(enriches_problem_with_property_description) + test(description_equal_to_message_is_suppressed) + test(whitespace_only_description_is_suppressed) + test(schema_validate_pretty_surfaces_property_description) + test(schema_validate_json_carries_description_field) + test(schema_validation_failed_block_renders_problem_description)' --color never
```

Result: 106 passed, 5288 skipped.

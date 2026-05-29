---
ready: false
agent: codex
model: ""
---

# Review: Schema Support in Claudine

## Findings

### Medium: schema-aware setter completions do not preserve declaration order within required/optional groups

- Requirement: schema-aware completion must list required properties before optional properties while preserving declaration order within each group.
- Implementation: `property_names` walks `shape.properties` directly and partitions names into `required` and `optional` vectors at `claudine/cli/src/completion/schema_completion.rs:93`. The local unit test documents the actual behavior as Darkmatter's effective schema storage order, "alphabetical when the `$schema` was authored as inline YAML", at `claudine/cli/src/completion/schema_completion.rs:411`. The integration test only proves required-before-optional ordering at `claudine/cli/tests/compose_schema_cli.rs:787`; it explicitly avoids asserting order between `topic` and `tier` at `claudine/cli/tests/compose_schema_cli.rs:819`.
- Impact: prompt authors cannot control completion ordering for required setters by arranging schema properties in the prompt. For example, a schema declared as `title`, then `status` can complete as `status=`, then `title=`, which violates the spec's declaration-order guarantee.
- Fix direction: preserve and expose the original SimplifiedSchema declaration order from Darkmatter, or have the completion path read the schema source in a way that keeps authored order before partitioning required and optional properties. Then strengthen tests so required `topic` before `tier` and optional property order are asserted for `compose`, with a smaller unit test covering the partition helper.
- Verification level: Level 1 is appropriate for this requirement because `__complete` output is a deterministic CLI protocol, not terminal rendering or keyboard-encoder behavior. Current Level 1 coverage is present but asserts only the weaker required-before-optional contract.

## Test Rigor Classification

- Direct `compose` schema validation for missing required values, setter-supplied values, invalid required values, invalid optional drops, templated required values, loop missing values, post-shell required validation, and visible dropped-optional warnings: Level 1 process coverage present.
- `inline-compose` prompt-property precedence and schema validation, including post-shell invalid required values: Level 1 process coverage present.
- `sequence` missing-property aggregation, setter-supplied success, unsupported non-TTY aggregation, and post-shell invalid required values before provider launch: Level 1 process coverage present.
- Interactive missing-property collection for string, enum, boolean, numeric retry, `--silent`, sequence dedupe, pre-launch sequence prompting, and status-report overlay/setter behavior: PTY coverage present in `level2_schema_prompt_pty.rs`. This is sufficient for the prompt/input behavior described here; no OS keyboard-encoder requirement is present, so Level 3 is not required.
- Schema-aware setter completions for required-before-optional ordering, supplied-property filtering, enum values, inline-compose, sequence, basename file globs, path-qualified globs, and negated path globs: Level 1 `__complete` coverage present. Declaration-order coverage within each group is missing, and the implementation appears not to satisfy it.

## Verdict

Not ready for production. The remaining gap is narrower than the earlier validation and interactive-mode issues, but it is a user-facing requirement in the spec and the current tests encode a weaker behavior than promised.

## Verification

- Source review of the current schema validation, interactive prompting, sequence, and completion changes.
- I did not run the test suite during this review.

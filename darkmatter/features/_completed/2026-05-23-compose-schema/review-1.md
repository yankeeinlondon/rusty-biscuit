---
ready: true
agent: codex
model: ""
---

# Review: Schema Validation in the Compose Pipeline

## Findings

### High: schema-preparation errors lose the actual diagnostic in the rendered block

`schema_validation::run` converts schema preparation failures into `MarkdownError::SchemaValidationFailed` with `problems: Vec::new()` and a useful `summary` such as `schema could not be prepared: ...` at [schema_validation.rs:47](../../lib/src/markdown/compose/schema_validation.rs) and [schema_validation.rs:59](../../lib/src/markdown/compose/schema_validation.rs). The `BlockError` renderer then explicitly ignores that summary via `_summary` and only renders the path, optional description, and generic hint at [blocks.rs:139](../../lib/src/markdown/errors/blocks.rs).

That means malformed or unresolved `$schema` cases do not satisfy the spec requirement to distinguish "schema could not be prepared" from "frontmatter did not satisfy the prepared schema." In the styled CLI path, authors may get an empty problem list with no root cause.

Suggested fix: render a preparation-specific body line when `problems.is_empty()` and `summary` starts with or otherwise represents schema preparation failure. Prefer preserving the underlying `SchemaError` as a source or separate variant if that matches the existing error layering.

Verification level present: Level 1 unit coverage exists for constructing validation-failure blocks, but there is no snapshot for a preparation failure with empty `problems`. Add Level 1 coverage for the rendered preparation-error block.

### High: the user-facing `md compose` regression is not verified at CLI level

The spec asks for the planner-prompt regression to assert that `md compose` exits non-zero with a styled `BlockError`, mentions `spec`, and does not surface `dirname`. The current regression at [mod.rs:4575](../../lib/src/markdown/compose/mod.rs) calls `Markdown::compose_with` in-process, so it does not exercise the binary, exit status, top-level error handler, stderr/stdout routing, or rendered CLI block.

This is not just a missing convenience test: the feature is user-facing through `md compose`, and the error rendering path is a key requirement. The in-process test is useful, but it is below the required verification for the CLI behavior.

Suggested fix: add a `darkmatter/cli/tests/` test that runs `md compose` on a fixture with `$schema`, `spec: ""`, and a shell-expansion sentinel. Assert non-zero exit, presence of `Schema validation failed` and `spec`, absence of `dirname`/sentinel output, and no sentinel side effect.

Verification level present: weaker than Level 1 for the CLI requirement because it does not spawn the binary. Required minimum: Level 1 process/PTY-style CLI coverage.

### High: styled terminal error requirements have only in-process snapshots, not real-terminal capture

The spec requires visible styling details: OSC8 source-file link, italic/dim description, red category labels, inverse property names, and line/column annotations. The current snapshots in [markdown_error.rs:129](../../lib/tests/error_snapshots/markdown_error.rs) verify renderer output in process, but they do not verify that the styled block survives through a real terminal renderer/emulator.

Per the review rubric, requirements like specific colors/styles and link rendering need Level 2 coverage when they are part of user-observable terminal behavior. Current coverage is Level 1 only.

Suggested fix: add a Level 2 terminal capture test for one representative schema validation failure block. It should run the binary in the existing real-terminal harness and capture pane text/SGR behavior where the harness supports it. If OSC8 cannot be captured as text, explicitly document and test the closest available signal.

Verification level present: Level 1 snapshots. Required minimum: Level 2 for the styled terminal block requirement.

### Medium: schema-validate parity is specified but not covered

The implementation reuses `DarkmatterSchemas::validate`, which is the right design, but there is no test proving `md compose` and `md schema validate` agree for the same document. The spec calls this out as an integration test because it guards against future drift in overrides, source handling, and CLI error mapping.

Suggested fix: add a CLI integration test with the same input file passed to `md compose` and `md schema validate`, asserting both fail for the same property and both succeed after the required property is supplied.

Verification level present: Level 1 unit/in-process tests for the compose stage and separate existing tests for `md schema validate`; no parity test.

### Medium: baseline cache safety is only hash-tested, not behavior-tested

`options_hash` includes `baseline_schema`, and [hashing.rs](../../lib/src/markdown/compose/cache/hashing.rs) has a focused unit test that different baselines produce different hashes. That covers the low-level key component, but the spec also asks for a baseline cache regression that composes the same transcluded child under different baselines and proves the second run does not reuse the first cached result.

Suggested fix: add an integration test with a persistent or run-local cache, a transcluded child, and two `ComposeOptions::with_baseline_schema(...)` values requiring different properties. Assert the second compose recomputes and returns the correct success/failure for its baseline.

Verification level present: Level 1 unit hash test only. Required minimum: Level 1 behavioral cache regression.

## Notes

The main implementation shape matches the spec: validation runs after `prepare_frontmatter_for_compose(...)`, before interpolation/shell expansion, document and baseline schemas flow through `DarkmatterSchemas::validate`, child compose inherits the baseline, and directive `set=` overlays are applied before child validation.

I attempted focused `cargo test` commands for schema validation, error snapshots, and baseline hash coverage, but the session contended on Cargo package/artifact locks and exceeded the non-interactive wait budget. I stopped those commands and did not count them as passing verification.

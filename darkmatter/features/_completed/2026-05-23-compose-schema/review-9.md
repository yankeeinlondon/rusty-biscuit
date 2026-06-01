---
ready: false
agent: codex
model: ""
---

# Review 9

## Findings

### Medium: Schema preparation errors lose their source error chain

The spec requires schema parsing, resolution, conversion, baseline merge, and validator-construction errors to flow through the compose error path while preserving the underlying `SchemaError` as the source. The current implementation converts those errors into `MarkdownError::SchemaValidationFailed` by formatting the diagnostic into `summary`:

- `darkmatter/lib/src/markdown/compose/schema_validation.rs:58`
- `darkmatter/lib/src/markdown/compose/schema_validation.rs:70`

But the `SchemaValidationFailed` variant has no `#[source]` field:

- `darkmatter/lib/src/markdown/types.rs:96`

That means CLI rendering can show the message, but programmatic callers using `std::error::Error::source()` cannot recover or inspect the original `SchemaError`. This is a direct API contract gap from the spec, and it also leaves no test proving source-chain preservation.

Recommended fix: add an optional source field for preparation failures, for example `source: Option<Box<crate::markdown::schemas::SchemaError>>` with the appropriate `#[source]` handling, or split preparation failures into a distinct error variant that carries the source. Add a Level 1 unit test that feeds a malformed or unresolved `$schema`, unwraps the compose error, and asserts that `Error::source()` exposes the schema error.

### Low: The compose module pipeline docs now contradict the actual stage order

The module-level compose docs say Schema Validation is step `0` and runs "before interpolation":

- `darkmatter/lib/src/markdown/compose/mod.rs:7`

The executable pipeline, the spec, and the updated skill/topic docs all place Schema Validation after Frontmatter Interpolation and before Frontmatter Shell Expansion:

- `darkmatter/lib/src/markdown/compose/mod.rs:528`
- `darkmatter/features/2026-05-23-compose-schema/spec.md`

This is comment drift; the code appears correct and the comment is wrong. It matters because this module-level doc is public-facing rustdoc for the compose API and describes exactly the ordering contract this feature changes.

Recommended fix: rewrite the Inline Pre list so Frontmatter Interpolation is first, Schema Validation is second, and the schema entry says it runs after `--set` / `--state` and frontmatter interpolation, but before frontmatter shell expansion.

## Test Rigor Notes

Requirements and strongest observed verification level:

- Always-on compose validation for document `$schema`: Level 1 unit/integration tests.
- No-op when no `$schema` and no baseline: Level 1 unit tests.
- Baseline schema injection and document-wins merge semantics: Level 1 unit tests.
- `--set` / `--state` effective-frontmatter ordering: Level 1 tests.
- Fail-fast before frontmatter shell expansion, including a CLI sentinel side-effect check: Level 1 CLI process tests.
- Recursive child validation after parent `set=` overlay: Level 1 integration tests.
- Baseline schema participation in option hashing and persistent transclusion cache keys: Level 1 tests.
- Styled `SchemaValidationFailed` block text, OSC8 source link, red category label, inverse property label, and dim/italic description rendering: Level 2 real-terminal capture tests.

No Level 3 requirement applies. This feature does not define OS keyboard, mouse, paste, IME, or terminal input-encoder behavior.

Test gap: there is no Level 1 assertion for preserving the schema-preparation source error chain, matching the medium finding above.

## Verification

I attempted focused verification with:

```bash
cargo test -p darkmatter schema_validation --color=never
cargo test -p darkmatter-cli --test compose_schema --color=never
```

Both commands were still compiling dependencies after the non-interactive session time budget. I terminated them rather than leave long-running sessions active. No test failure was observed, but verification is inconclusive.

## Recommendation

Not ready for production until schema-preparation failures preserve their underlying `SchemaError` source and the drifted compose rustdoc is corrected.

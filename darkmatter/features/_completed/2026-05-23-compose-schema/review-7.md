---
ready: false
agent: codex
model: ""
---

# Review 7

## Findings

### Medium: Perf reports list Schema Validation before Frontmatter Interpolation

The implementation executes Schema Validation after the first frontmatter interpolation pass, which matches the amended spec:

- `darkmatter/lib/src/markdown/compose/mod.rs:511`
- `darkmatter/lib/src/markdown/compose/mod.rs:541`

However, the public perf stage ordering says the opposite. `PerfMetricKind` and `ComposeStage` both document that variants are listed in pipeline execution order, but both put `SchemaValidation` before `FrontmatterInterpolation`:

- `darkmatter/lib/src/markdown/compose/perf.rs:11`
- `darkmatter/lib/src/markdown/compose/perf.rs:15`
- `darkmatter/lib/src/markdown/compose/perf.rs:61`
- `darkmatter/lib/src/markdown/compose/types.rs:1481`
- `darkmatter/lib/src/markdown/compose/types.rs:1485`

That makes `ComposePerfReport.metrics` misrepresent the pipeline order whenever `with_perf(true)` is used. This is a public diagnostic surface and can mislead callers trying to understand why schema validation saw a particular frontmatter value.

Recommended fix: move `SchemaValidation` after `FrontmatterInterpolation` in both `PerfMetricKind` and `ComposeStage`, and add a focused Level 1 test that asserts the perf metric order is `FrontmatterInterpolation -> SchemaValidation -> FrontmatterShellExpansion`.

Verification level: Level 1 is appropriate because this is an in-process API/reporting contract, not terminal rendering or keyboard behavior.

## Test Rigor Notes

The functional schema-validation requirements are covered at Level 1 through unit/in-process compose tests and CLI process tests:

- no-op when no schema/baseline exists
- document `$schema` success/failure
- baseline schema merge and document-overrides-baseline behavior
- post-override validation behavior
- recursive child validation with `set=` overlay
- fail-fast before frontmatter shell expansion
- baseline schema participation in cache keys and persistent cache reuse

The styled `SchemaValidationFailed` block has Level 2 coverage in `darkmatter/cli/tests/level2_errors.rs`, including OSC8 source links, visible property text, dim/italic description styling, red category labels, and inverse property labels. That is the correct level for terminal-rendered glyph/style behavior.

I found no Level 3 requirement in this feature. The spec does not define OS keyboard, paste, IME, mouse, or modifier-key behavior.

## Verification

I attempted focused Cargo verification:

- `cargo test --color=never -p darkmatter compose_schema`
- `cargo test --color=never -p darkmatter schema_validation`
- `cargo test --color=never -p darkmatter-cli --test compose_schema`

These jobs contended on Cargo locks and then the active compile exceeded the non-interactive session time budget, so I terminated them. No test failure was observed; verification is inconclusive.

## Recommendation

Not ready for production until the perf/report ordering is corrected or the public docs stop promising execution order. The core compose-schema behavior otherwise appears aligned with the amended spec and has the right testing levels for the user-observable surfaces in scope.

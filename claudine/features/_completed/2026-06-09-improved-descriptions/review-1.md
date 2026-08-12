---
ready: false
agent: codex
model: ""
---

# Review: Catalog Drift Control & Runtime-Accessible Descriptions

## Findings

### High: Level 2 context expression rendering is currently failing

`claudine context --expressions` has a real-terminal rendering regression. The Level 2 test `level2_context_expressions_list_reserves_right_margin_in_tmux` fails because the captured tmux pane contains no `- ` list markers where the test expects a wrapped unordered list section.

Evidence:

- `claudine/cli/tests/level2_context_capture.rs:250` checks for real-terminal list markers and wrapped continuation lines.
- The rerun failed at `claudine/cli/tests/level2_context_capture.rs:274` with `expected at least one '- ' list marker`.
- The captured pane shows expression tables, but no list marker section.

Verification level: Level 2, appropriate for this user-observable terminal layout requirement. Because the strongest applicable test is failing, this feature is not production-ready.

### High: fuzzy suggestions ignore the spec's quality gate and always return a suggestion

The spec requires `suggest(...)` to omit matches whose normalized edit distance is greater than `max(2, normalized_query.len() / 3)`. The current implementation scores every descriptor and returns the first `max` entries unconditionally.

Evidence:

- `darkmatter/lib/src/catalog/mod.rs:43` starts `suggest`.
- `darkmatter/lib/src/catalog/mod.rs:48` normalizes the query.
- `darkmatter/lib/src/catalog/mod.rs:57` sorts all candidates.
- `darkmatter/lib/src/catalog/mod.rs:58` returns the first `max` candidates with no distance threshold.
- `darkmatter/lib/src/markdown/compose/expression/mod.rs:540` uses this for unknown-function diagnostics, so unrelated typos can get authoritative-looking did-you-mean output.

This is a functional gap in the headline runtime-accessible description path. It also means the test at `darkmatter/lib/src/markdown/compose/expression/mod.rs:680` does not prove its name: it only checks the unknown-function prefix, not that no close-match suggestion is emitted.

Verification level: Level 1 is sufficient for this pure function and diagnostic behavior, but coverage is incomplete.

### High: context typo diagnostics do not cover all spec-required composition paths

The spec requires parser-aware `ctx.*` typo diagnostics for body interpolation, composed frontmatter expressions, condition attributes, and loop/sequence condition expressions where Darkmatter parses expressions. The implementation only wires warnings through interpolation rewrite.

Evidence:

- Body/frontmatter string interpolation calls `collect_context_warnings` at `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs:93`.
- Whole-value scalar frontmatter interpolation bypasses that warning path: `interpolate_value` calls `whole_value_scalar` first at `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs:199`, and `whole_value_scalar` parses/evaluates directly at `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs:222`.
- Condition evaluation parses and evaluates directly at `darkmatter/lib/src/markdown/compose/conditions.rs:138` and `darkmatter/lib/src/markdown/compose/conditions.rs:146`, with no warning collection.
- The only direct tests I found cover interpolation text: `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs:469` and `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs:487`.

This leaves user-visible typo assistance incomplete for documented surfaces. Silent-null evaluation remains correct, but the warning layer is not applied consistently.

Verification level: Level 1 is sufficient for these parser/compose diagnostics, but required cases are missing.

### Medium: `ExampleVerification` was not implemented, so display-only examples are not machine-auditable

The spec defines `Example { invocation, result, verification }` plus `ExampleVerification::{Executable, TypeShapeOnly, DisplayOnly(reason)}`. The implementation only stores `invocation` and `result`.

Evidence:

- `darkmatter/lib/src/catalog/mod.rs:9` defines `Example`.
- `darkmatter/lib/src/catalog/mod.rs:11` through `darkmatter/lib/src/catalog/mod.rs:16` contain only `invocation` and `result`.
- Expression and semantics descriptors use `None` plus comments for display-only cases, for example `darkmatter/lib/src/markdown/compose/expression/catalog.rs:562` and `darkmatter/lib/src/markdown/compose/expression/semantics.rs:636`.
- Effect examples are described as "verified" in field docs at `darkmatter/lib/src/effects/catalog.rs:36`, but no verification intent or display-only reason exists in data.

This weakens the core drift-control model: tests cannot assert that every descriptor declares its verification intent, and display-only reasons are comments rather than runtime-readable metadata.

Verification level: Level 1 is sufficient for the descriptor shape and catalog assertions, but the required data model is absent.

## Additional Notes

The plan's validation command `cargo test -p darkmatter-lib ...` is not reproducible because the workspace package is named `darkmatter`, not `darkmatter-lib`. I confirmed this with Cargo metadata.

Targeted verification performed:

- `cargo test -p claudine-cli context --color=never` passed the Level 1 context-related unit/integration tests it reached, then selected Level 2 tests and was stopped after a failure plus many long-running real-terminal tests.
- `cargo test -p claudine-cli --test level2_context_capture level2_context_expressions_list_reserves_right_margin_in_tmux --color=never -- --nocapture` failed reproducibly.

## Production Readiness

Not ready. The feature has at least one failing Level 2 rendering test and multiple spec-required behavior gaps in the runtime suggestion and context diagnostic surfaces.

---
agent: codex
model: ""
ready: false
---

# Review - Iteration 1

## Findings

### High - `inline-compose` loops with empty bodies now fail during seed resolution

`run_inline_compose_inner` routes loop seeding through `run_loop_with_overrides`, which calls `build_loop_seed` at `claudine/lib/src/composition/loop_engine.rs:231`. `build_loop_seed` always calls `prepare_direct` at `claudine/lib/src/composition/loop_engine.rs:236`, but the inline loop executor correctly uses `prepare_inline_with_schema` at `claudine/cli/src/commands/compose.rs:1124`.

That mode mismatch is user-visible. An inline composition document whose prompt lives in frontmatter and whose body is empty is a valid inline-compose shape, but the seed pass treats it as chained compose and fails with `ComposedBodyEmpty` before iteration 1. I confirmed this with `target/debug/claudine inline-compose --goose <fixture>`: no provider call was made and the error reported `Mode: chained (compose)`.

This violates the spec's "library and CLI loop entrypoints share one seeding path; no behavioral drift" criterion for the inline CLI path, because the shared seed path is not parameterized by composition mode. Fix by making seed preparation mode-aware, or by adding an inline seed builder that composes the `prompt` frontmatter the same way `prepare_inline_with_schema` does.

Verification level: current coverage is Level 1, but the existing `inline_compose_loop_runs_iterations` test only covers a non-empty body. Add a Level 1 CLI regression for an inline loop with `prompt:` frontmatter and an empty body.

### High - Conditions using `doc.<path>` lose read-only control values after seeding

`extract_control_variables` excludes `doc` as a reserved namespace at `claudine/lib/src/composition/loop_config.rs:135`, so a condition like `while: "doc.counter < doc.total"` does not lift `total` into the loop-owned state unless another action also targets `total`. However `LoopExpressionLookup::get` resolves `doc.*` from the loop state map at `claudine/lib/src/composition/loop_expression.rs:92`, not from the original/effective document frontmatter.

After this change, the seed for this document contains `counter` from the `increment(counter)` action but omits the read-only `total` value. The condition then evaluates false and the loop silently runs zero iterations. I confirmed this with a fake `goose` CLI: a document with `counter: 0`, `total: 2`, `while: "doc.counter < doc.total"`, and `increment(counter)` exited successfully without invoking the provider.

This regresses the loop expression contract for the `doc` namespace and creates a silent skip rather than an actionable error. Either collect `doc.<head>` references as control variables, or change `doc.*` lookup during loop evaluation to read through a merged view that includes resolved read-only effective frontmatter without pinning derived presentation keys into overrides.

Verification level: current coverage is Level 1 for `doc.*` lookup in isolation, but there is no Level 1 seeded-loop test that combines `doc.*` in the condition with the control-variable seed. Add one before marking this ready.

## Test Rigor

- Core repro behavior (`phase` and `total_phases` expression-defined, `increment(phase)`, body shows `Implement Phase N of 6`, derived `pass_icon` stays live): covered at Level 1 by `seeded_loop_repro_runs_to_completion_with_live_derived_variable`. Level 1 is appropriate because this is pure composition state flow, not terminal encoder behavior.
- Error value excerpts for increment/decrement failures: covered at Level 1 by structural unit tests. Level 1 is appropriate for the data contract; no terminal rendering requirement requires Level 2 here.
- CLI compose schema regression: covered at Level 1 by `compose_loop_missing_required_surfaces_typed_missing_properties`.
- Missing coverage: inline-compose empty-body loop seeding and seeded `doc.*` conditions, both Level 1 gaps.

## Verification Run

- `cargo test -p claudine composition::loop_config --color=never` - passed.
- `cargo test -p claudine composition::loop_engine --color=never` - passed.
- `cargo test -p claudine composition::loop_actions --color=never` - passed.
- `cargo test -p claudine composition::loop_expression --color=never` - passed.
- `cargo test -p claudine-cli --test compose_schema_cli compose_loop_missing_required_surfaces_typed_missing_properties --color=never` - passed.
- `cargo test -p claudine-cli --test loop_cli inline_compose_loop_runs_iterations --color=never` - passed, but it does not cover the empty-body inline shape.

## Summary

The main direct-compose repro path is implemented and tested well at the right level. I would not ship this yet because the shared seed helper is direct-compose-only and because the control-variable extractor drops `doc.*` dependencies that the loop evaluator still expects to resolve from state.

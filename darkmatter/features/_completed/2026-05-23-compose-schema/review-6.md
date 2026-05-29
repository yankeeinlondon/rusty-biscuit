---
ready: false
agent: codex
model: ""
---

# Review 6

## Findings

### High: Schema validation runs after frontmatter interpolation, contrary to the spec

The spec's central placement requirement is: apply `--set` / `--state`, then run Schema Validation, then run frontmatter interpolation and frontmatter shell expansion. The implementation does the opposite for interpolation: `run_compose_pipeline_internal` executes `frontmatter_interpolation::interpolate_frontmatter(...)` first, then calls `schema_validation::run(...)`.

References:

- `darkmatter/lib/src/markdown/compose/mod.rs:510`
- `darkmatter/lib/src/markdown/compose/mod.rs:540`
- `darkmatter/lib/src/markdown/compose/schema_validation.rs:3`

This is not just a documentation mismatch. It changes the contract from validating the effective authored frontmatter after overrides to validating a partially transformed frontmatter state. It also conflicts with the updated Darkmatter skill and the spec text that say validation runs before interpolation. If the intended behavior has changed, the feature spec and skill need to be amended; otherwise the stage should move back to immediately after `prepare_frontmatter_for_compose(...)`.

Verification level: Level 1 tests cover some fail-fast behavior, but they do not verify the required ordering. Add a focused in-process test where an invalid frontmatter value would be changed by frontmatter interpolation; the schema error must be based on the pre-interpolation value per the current spec.

### High: Shell-dependent schema violations can bypass compose validation and still run shell expansion

`schema_validation::run` validates, then filters out every validation problem whose top-level field contains `$(`. If all problems are filtered, the function returns `Ok(())`, allowing frontmatter shell expansion to execute.

References:

- `darkmatter/lib/src/markdown/compose/schema_validation.rs:76`
- `darkmatter/lib/src/markdown/compose/schema_validation.rs:83`
- `darkmatter/lib/src/markdown/compose/schema_validation.rs:96`

That violates the spec's hard-error requirement: schema violations should fail before interpolation or shell expansion, and the CLI has no later compose-time revalidation step. A document like this can avoid the schema error path and run the command:

```yaml
---
$schema:
  spec: 'file(required)'
spec: "$(touch /tmp/schema-ran-shell && printf missing-file)"
---
```

The current tests prove shell expansion does not run when an independent field (`spec: ""`) fails validation, but they do not cover a failing property whose own value contains `$(`. The downstream Claudine revalidation mentioned in comments is outside this feature's stated `md compose` contract.

Verification level: Level 1 is sufficient for this user-observable CLI behavior. Add a binary or in-process test with a sentinel command in the schema-constrained property itself and assert `SchemaValidationFailed` plus no sentinel side effect.

### High: Child schema validation failures are downgraded to transclusion warnings by default

The spec says schema violations are hard errors and recursive compose validates every Markdown document. In the transclusion phase, any child compose error that is not structural is converted into a warning when `fail_fast` is false. The integration test for a child schema failure explicitly sets `with_fail_fast(true)` to avoid that downgrade.

References:

- `darkmatter/lib/src/markdown/compose/mod.rs:1029`
- `darkmatter/lib/src/markdown/compose/mod.rs:1039`
- `darkmatter/lib/src/markdown/compose/mod.rs:1042`
- `darkmatter/lib/src/markdown/compose/mod.rs:4826`

Because `ComposeOptions::new()` defaults `fail_fast` to false, a parent can transclude a child with an invalid `$schema`, receive only a warning, and continue composing. That is inconsistent with "Schema violations are hard errors" and with the expected `md compose` fail-fast behavior for every composed Markdown document.

Verification level: Level 1 is sufficient. Add a default-options recursive compose test where a child is missing a required schema property and assert the parent compose returns `MarkdownError::SchemaValidationFailed` without requiring `with_fail_fast(true)`.

## Test Rigor Notes

Functional schema behavior is mostly Level 1, which is appropriate: unit tests cover `schema_validation::run`, integration tests spawn the `md` binary, and cache behavior is exercised through compose. Styled error rendering has Level 2 coverage in `darkmatter/cli/tests/level2_errors.rs` for OSC8 links, SGR styling, and the schema-validation bullet, which matches the terminal-rendering requirements.

I did not find a Level 3 requirement in this feature: no OS keyboard input, paste, IME, mouse, or modifier-key behavior is specified.

## Verification

I attempted `cargo test --color=never -p darkmatter-cli --test compose_schema`, but the workspace was still compiling dependencies after roughly 90 seconds, so I terminated it to avoid a long non-interactive build. The resulting SIGTERM errors are from my termination, not a test failure.

## Recommendation

Not ready for production. The implementation is close in API shape and error rendering, but the core validation timing and hard-error semantics diverge from the spec in ways that can allow invalid schema-constrained documents to continue composing.

## Resolution (2026-05-28)

All three findings were verified as accurate descriptions of how the code diverges from `spec.md` as originally written. The divergence is **deliberate**, not a bug: the implementation validates *after* frontmatter interpolation (so schema-constrained fields can derive from `{{ }}` templates), defers problems on fields still holding `$(...)` (re-validated downstream by claudine after shell expansion), and follows the standard transclusion error policy for child failures (hard error under `fail_fast`/structural, warning otherwise).

Per maintainer decision, the divergence was resolved by **amending the spec and documentation to match the implemented behavior**, rather than reverting the code to the original spec. No production code was changed. Updated:

- `spec.md` — Decisions #1, #2, #5, #6; the pipeline-placement diagram; the `compose::mod.rs` / `compose::perf.rs` touchpoints; and the recursive-compose test note.
- `darkmatter/docs/topics/schema-definition.md` — the Compose Pipeline Integration intro, pipeline diagram, and behaviour bullets.
- `.claude/skills/darkmatter/SKILL.md` and `compose.md` — the Inline Pre stage ordering and Schema Validation summary (SKILL.md `hash:` regenerated).

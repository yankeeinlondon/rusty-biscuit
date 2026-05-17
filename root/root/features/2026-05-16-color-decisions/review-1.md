---
ready: false
agent: codex
model: ""
---

# Review: CodeRenderer Terminal Color Context

## Summary

The implementation lands the core shape from the spec: `renderable::color`
now has terminal color capability descriptors, `CodeRenderer` receives
`TerminalCodeContext`, and `biscuit-terminal` maps its render context into the
hook at the code-node call site. The behavior-changing code is small and
mostly correct.

I would not mark this ready for production yet. The main gaps are test
placement and test rigor against the explicit pass-through acceptance
criteria, plus one documentation mismatch that can mislead the future
darkmatter `CodeRenderer` implementor.

## Findings

### High: Required pass-through tests are not in `biscuit-terminal`

Spec §9 requires the pass-through tests to be in `biscuit-terminal`. The
implementation adds them in
`darkmatter/lib/tests/yaml_block_parity.rs:217`, not in
`biscuit-terminal/lib/src/render_tree/render.rs` or
`biscuit-terminal/lib/tests`.

That means `cargo test -p biscuit-terminal` does not verify the new
`biscuit-terminal` call-site contract, even though the changed production
code is in `biscuit-terminal/lib/src/render_tree/render.rs:637`.

Verification level: Level 1 is the correct level for this requirement because
it is an in-process trait-call contract, not real terminal rendering or input
encoding. Current strongest coverage is Level 1 but attached to the wrong
crate/test suite, so the spec's verification requirement is not met.

Recommendation: move or duplicate the capturing `CodeRenderer` pass-through
tests into the `biscuit-terminal` render-tree tests, constructing a plain
`RenderNode::code(...)` directly instead of going through `YamlBlock`.

### High: The "conflicting ambient env vars" criterion is not actually tested

Spec §9(d) requires values to match a manually built context with no influence
from conflicting ambient environment variables. The test named
`context_ignores_ambient_environment` does not set any conflicting environment
variables; it explicitly comments that it is not doing so at
`darkmatter/lib/tests/yaml_block_parity.rs:375`.

As written, the test only proves the renderer passes explicit context values
in a normal ambient environment. It does not prove the regression the spec is
trying to prevent: accidental re-detection through `COLORTERM`, `NO_COLOR`,
`COLORFGBG`, or similar.

Verification level: Level 1 is appropriate here because this is still an
in-process renderer contract. The strongest current verification is below
Level 1 for this specific requirement because the manufactured conflicting
inputs are absent.

Recommendation: add a serial/env-isolated test in `biscuit-terminal` that
sets conflicting env vars, builds an explicit `TerminalRenderContext`, renders
a code node through a capturing renderer, and asserts the captured
`TerminalCodeContext` follows the context, not the environment.

### Medium: `ColorMode::Unknown` docs contradict the settled fallback rule

Spec D-6 says `Unknown` must be resolved by a code renderer against its own
configured option, defaulting to `Dark` when there is no configured option.
The docs in `renderable/src/color/capability.rs:66` instead tell
implementors to use a conservative palette that works on both light and dark
backgrounds.

That is a subtle but important contract mismatch for the future darkmatter
implementation. The spec exists specifically to prevent code renderers from
inventing ad-hoc capability decisions, and this doc encourages a different
fallback policy than D-6.

Recommendation: update the `ColorMode::Unknown` rustdoc to mirror D-6:
unknown is a faithful signal; do not run ambient detection; resolve through
configured renderer options, otherwise default to dark.

### Medium: A test-only constructor was added to public API

`TerminalRenderOptions::from_context` was added at
`biscuit-terminal/lib/src/render_tree/options.rs:248` and documented primarily
as useful for testing. The spec does not require this new public constructor,
and `TerminalRenderOptions` already exposes its `context` field publicly, so
tests can construct options via existing APIs and mutate the context.

This is not a correctness bug, but it expands public API for a narrow testing
need. If the constructor is intended as a real user-facing API, it should be
documented as such. Otherwise, keep the change surgical and avoid adding it.

Recommendation: either remove `from_context` and use existing public fields in
tests, or rewrite the docs to present it as a supported explicit-context
rendering API rather than a test helper.

## Requirement Coverage

- `ColorDepth`, `ColorMode`, and `TerminalCodeContext` exist in
  `renderable::color`.
- `CodeRenderer::render_terminal_code` now takes `TerminalCodeContext`; the
  width-only signature is gone.
- `biscuit-terminal` provides `From` impls and populates the context from
  `available_width`, `color_depth`, and `color_mode`.
- `render_browser_code` appears unchanged.
- Required Level 1 pass-through coverage is incomplete because tests are in
  the wrong crate and the ambient-conflict case does not actually create
  conflicting ambient inputs.

## Verification Performed

- `cargo test -p renderable --color=never` passed.
- `cargo test -p darkmatter --test yaml_block_parity --color=never` passed.
- `cargo clippy -p renderable --all-targets --color=never -- -D warnings`
  passed.
- `cargo clippy -p biscuit-terminal --all-targets --color=never -- -D warnings`
  passed.
- `cargo clippy -p darkmatter --test yaml_block_parity --color=never -- -D warnings`
  passed.
- `cargo test -p biscuit-terminal --color=never` failed in existing
  `level1_terminal_init::terminal_new_cascade_produces_consistent_fields_in_pty`
  with `expected TTY detection inside PTY`; this does not appear caused by
  the color-context implementation, but it means the full required terminal
  crate test command was not green in this environment.

## Production Readiness

Not ready. The implementation is close, but the feature should not be marked
production-ready until the specified Level 1 pass-through tests live in
`biscuit-terminal`, the ambient-conflict test actually sets conflicting
environment values, and the `Unknown` fallback documentation matches the spec.

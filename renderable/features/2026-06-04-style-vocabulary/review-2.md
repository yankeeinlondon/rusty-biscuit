---
ready: false
agent: codex
model: ""
---

# Review: Style Vocabulary

## Findings

### High: Inline-node `Layout` is still accepted as a warning, contrary to the spec

The spec says `Layout` remains block-only and that tree validation must continue rejecting layout attrs on inline nodes. The implementation still treats this as a warning: `check_node` records `Severity::Warning` when `is_inline_kind(&node.kind)` has a layout, and the regression test explicitly asserts `ensure_valid(&root).is_ok()` for a `Text` node carrying `Layout::default()` ([renderable/src/tree/validate.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/tree/validate.rs:333), [renderable/src/tree/validate.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/tree/validate.rs:686)).

This leaves serialized trees with inline geometry valid at the validation gate, even though the browser renderer then drops inline layout (`node_attributes` applies layout only when `!inline`) and the vocabulary contract says inline spans have no padding or block width ([renderable/src/tree/render/browser.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/tree/render/browser.rs:2240)). Change the finding to `Severity::Error`, update the test to require `ensure_valid` failure, and keep style-on-inline behavior unchanged.

Verification level: Level 1 is sufficient because this is a structural validation/API contract. Current strongest verification is Level 1, but it verifies the opposite behavior.

### Medium: Active docs/source still contain forbidden deleted-vocabulary tokens

Acceptance criterion 3 requires `rg 'FillBand|FillIntensity'` over `renderable/` to return nothing outside historical feature docs. It still matches active source docs and active docs/README, including `renderable/src/style.rs`, `renderable/docs/layout-and-style.md`, `renderable/README.md`, and `.claude/skills/renderable/style.md`.

The types are deleted from Rust APIs, so this is not a compile break, but it misses the explicit cleanup check and keeps the old type names alive in current guidance. Reword active guidance to say "former subtle/pronounced fill tints" or similar without spelling `FillBand` / `FillIntensity`; leave the exact names only in historical feature docs/specs if needed.

Verification level: documentation/API cleanup contract. No Level 2 or Level 3 verification is required.

## Test Rigor

For this spec's implemented scope, Level 1 is the appropriate verification level: it defines Rust vocabulary, serde/defaulting, validation, and docs. Terminal/browser rendering of `padding`, `width`, and `fit-content` is explicitly deferred to renderer-folds, so Level 2 real-terminal capture and Level 3 OS input tests are not required here.

Covered at Level 1: `Layout`/`Style` defaults, serde round-trips, old `Layout` payload defaulting, `padding` and `width` validation, `Width` snake_case tags, `Background::subtle()` / `pronounced()` adaptive colors, and downstream compile compatibility.

## Verification

- `GIT_TERMINAL_PROMPT=0 cargo test --color=never -p renderable --lib` passed: 389 tests.
- `GIT_TERMINAL_PROMPT=0 cargo check --color=never -p renderable -p biscuit-terminal -p darkmatter` passed.

## Readiness

Not ready for production. The core type work compiles and is well covered at Level 1, but the validator still accepts an explicitly disallowed tree shape, and the documented deleted-vocabulary cleanup check still fails.

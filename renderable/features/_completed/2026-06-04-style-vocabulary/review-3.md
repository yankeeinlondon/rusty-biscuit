---
ready: true
agent: codex
model: ""
---

# Review: Style Vocabulary

## Findings

No production-blocking findings.

The prior review blockers are addressed:

- Inline `Layout` on inline nodes is now an error, and `ensure_valid` rejects it.
- Default tests now pin every `Layout` field, including `word_wrap`, and every `Style` field.
- Active renderable docs/skills no longer contain the forbidden `FillBand` / `FillIntensity` tokens outside historical feature material.

## Test Rigor

Level 1 is the appropriate verification level for this spec's implemented scope. The feature defines Rust vocabulary, serde/defaulting, validation, public exports, and documentation. Terminal/browser rendering of `padding`, `width`, and `fit-content` is explicitly deferred to the renderer-folds spec, so Level 2 real-terminal capture and Level 3 OS input tests are not required for production readiness here.

Verified at Level 1:

- `Layout` fields/defaults: `margin`, `padding`, `width`, `max_width`, `alignment`, `word_wrap`.
- `Edges` replacement for the former renderable `Margin` type, with no public alias found.
- `Width` serde tags: `auto`, `fit_content`, and `fixed`.
- Old serialized `Layout` payloads without `padding` / `width` default correctly.
- `Layout::validate` covers `margin`, `padding`, `width`, and `max_width`.
- Inline-node `Layout` fails tree validation.
- `Style` no longer stores `fill`; old `fill` keys deserialize as ignored unknown fields.
- `Background::subtle()` / `pronounced()` produce the expected adaptive tint values.
- `Style::is_empty()` treats background-only styles as non-empty.

## Verification

- `GIT_TERMINAL_PROMPT=0 cargo test --color=never -p renderable --lib` passed: 389 tests.
- `GIT_TERMINAL_PROMPT=0 cargo check --color=never -p renderable -p biscuit-terminal -p darkmatter` passed.
- `rg -n 'no padding|has no padding|lacks padding|FillBand|FillIntensity|renderable::layout::Margin|layout::Margin\b' .claude/skills/renderable renderable/docs renderable/README.md biscuit-terminal/README.md -S` returned no matches.

## Readiness

Ready for production for the style-vocabulary spec. The remaining rendered box behavior is intentionally owned by the sibling renderer-folds spec, not this vocabulary change.

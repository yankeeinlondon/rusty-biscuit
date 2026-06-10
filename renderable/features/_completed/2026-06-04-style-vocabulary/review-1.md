---
ready: false
agent: codex
model: ""
---

# Review: Style Vocabulary

## Findings

### High: Required renderable docs and skills still describe the deleted vocabulary

The implementation updates the core Rust types, but the required documentation/skill sweep is incomplete. The spec explicitly requires updating `.claude/skills/renderable/layout.md`, `.claude/skills/renderable/style.md`, `renderable/docs/layout-and-style.md`, and READMEs that describe `Fill`, `Margin`, or missing padding. Those files still present `Margin` and `Fill` as active APIs:

- `.claude/skills/renderable/SKILL.md` still lists `Margin` and `Fill` in the module table.
- `.claude/skills/renderable/layout.md` still imports and constructs `Margin`, says `Layout` has margins/alignment/max-width/wrapping only, and says validation covers only `margin`/`max_width`.
- `.claude/skills/renderable/style.md` still has an active `Fill` section and says `Style`, `Border`, and `Fill` derive serde.
- `renderable/docs/layout-and-style.md` still documents `pub margin: Margin`, no `padding`/`width`, `pub fill: Option<Fill>`, and terminal `FillBand` rendering.
- `renderable/README.md` still lists `Margin`, `Fill`, `FillIntensity`, and `FillBand`.

This violates acceptance criterion 8 and leaves future agents/users with instructions that no longer compile after the API change.

Verification level: documentation/API contract. No Level 2 or Level 3 terminal verification is required, but this needs a content check such as `rg -n 'FillBand|FillIntensity|renderable::layout::Margin|Layout.*no padding|has no padding' .claude/skills/renderable renderable/docs renderable/README.md`.

## Test Coverage Notes

The core Rust vocabulary changes have appropriate Level 1 coverage for this spec's scope: defaults, serde round-trips, old `Layout` payload compatibility, `padding`/`width` validation, and `Background::subtle()` / `pronounced()` RGB values are tested in-process.

No Level 2 or Level 3 tests are required for this feature as specified, because terminal/browser rendering of `padding`, `width`, and `fit-content` is explicitly deferred to the renderer-folds spec. If this feature is expanded to claim rendered padding/background/fit-content behavior, that would require Level 2 real-terminal/browser-style verification.

One small Level 1 gap remains: the defaulting contract says every `Layout` / `Style` field should be pinned. Current tests cover the important fields, but `Layout::default().word_wrap == WordWrap::None` and the full `Style` field-by-field default shape are mostly covered indirectly through `is_empty()`.

## What Passed

- `cargo test --color=never -p renderable --lib` passed: 387 tests.
- `cargo check --color=never -p biscuit-terminal -p darkmatter -p renderable` passed in 1m08s.

## Readiness

Not ready for production. The code compiles and the Level 1 implementation coverage is largely in place, but the feature does not meet the required documentation/skill acceptance criteria.

---
ready: false
agent: codex
model: ""
---

# Review 5

## Findings

### High: detailed moved/reordered section reports drop simultaneous content changes

The spec says a `detailed` comparison can report, per section, whether the heading changed, whether the content changed, whether the level changed, and whether the section moved relative to siblings. The current detailed explanation keeps `content_changed` for promotions/demotions, but drops it for `Moved` and `Reordered` reports.

Relevant code:

- `darkmatter/lib/src/markdown/hash/explain.rs:186` defines `Reordered` without a `content_changed` field.
- `darkmatter/lib/src/markdown/hash/explain.rs:188` defines `Moved` without a `content_changed` field.
- `darkmatter/lib/src/markdown/hash/explain.rs:234` renders a reordered section as only `reordered, now appears before ...`.
- `darkmatter/lib/src/markdown/hash/explain.rs:237` renders a moved section as only the move phrase.
- `darkmatter/lib/src/markdown/hash/explain.rs:626` computes `content_changed`, but `darkmatter/lib/src/markdown/hash/explain.rs:663` and `darkmatter/lib/src/markdown/hash/explain.rs:671` return `Moved` / `Reordered` before that content signal can be rendered.

That means a document can change a section's placement and edit that same section's prose, but `md hash --diff` / `md hash --save` will only tell the user about the placement change. For example, moving `Caveats` under a different parent while also rewriting its content would render as moved beneath the new parent, with no indication that the section body changed too.

Fix by carrying a `content_changed: bool` on `Moved` and `Reordered` (or by emitting a second line for the same section) and adding Level 1 regression tests for:

- same-heading section moved to a different parent and content changed
- same-heading sibling reordered and content changed

Verification level: Level 1 is appropriate here because this is pure library/CLI classification and plain-text rendering. No Level 2 or Level 3 terminal coverage is required for this hashing requirement. Current Level 1 tests cover content changes, moves, and reorders separately, but not the combined moved/reordered-plus-content-edited cases.

## Production Readiness

Not ready for production. The core hashing, save, ignore-policy, malformed-baseline, and CLI paths are substantially implemented, and this feature does not need real-terminal Level 2/Level 3 verification. The remaining gap is still user-facing: detailed explanations can omit a real content edit when the same section also moves or reorders.

## Verification Notes

I attempted focused test runs:

- `cargo test -p darkmatter --lib markdown::hash -- --nocapture`
- `cargo test -p darkmatter-cli test_hash_ --test cli -- --nocapture`

Both were still compiling dependencies and blocking on cargo work after the useful review window, so I stopped the cargo processes. No failing test output was observed before stopping them.

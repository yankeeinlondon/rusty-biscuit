---
ready: false
agent: codex
model: ""
---

# Review: Block Extension - HR-Attribute Lift

## Findings

### High - The legacy HR processor is still active and public, so the lift is not complete

The spec's success criteria include retiring `SpannedRuleProcessor`/the old HR path from the active chain and moving HR-attribute handling out of inline-span transport. The render-tree fold now uses `BlockExtensionProcessor` before folding, which is the right direction, but the legacy `RuleProcessor` remains publicly exported from `darkmatter::markdown::block` and remains the active HR-attribute processor for both legacy HTML and terminal renderers:

- `darkmatter/lib/src/markdown/block/mod.rs` still exports `RuleProcessor`.
- `darkmatter/lib/src/markdown/block/rule_processor.rs` still documents and implements paragraph-to-`InlineEvent::HorizontalRule` rewriting.
- `darkmatter/lib/src/markdown/output/html.rs` and `darkmatter/lib/src/markdown/output/terminal.rs` still instantiate `RuleProcessor::new(InlineStyleProcessor::new(parser))`.

That may be an intentional bridge while legacy renderers remain public, but it means this implementation has not actually completed the architectural goal as written. The sibling inline-span deletion still cannot delete the old inline event HR path globally without either cutting the legacy renderers over or clearly narrowing the spec to "render-tree path only".

Recommended fix: either complete the cutover so all active render paths use the block-extension processor, or update the spec/plan to explicitly preserve `RuleProcessor` as a legacy-only compatibility path and add a follow-up gate for removing it.

### High - Required byte-identical/parity behavior is not verified at the level stated by the spec

The spec says to preserve byte-identical output for existing HR-attribute fixtures, including `mark_dim_hr`. The current tests verify useful behavior, but not that requirement:

- `render_tree_parity_hr_attributes_spanned` checks only visible text and explicitly treats exact HR styling as an acceptable formatting difference.
- `mark_dim_hr` exists in `darkmatter/lib/benches/migration_parity.rs`, but benchmarks are not assertions.
- The block-extension tests are Level 1 parser/fold tests and do not compare the full legacy and tree outputs byte-for-byte.

Verification level present:

- HR paragraph recognition and source-range policy: Level 1.
- Warning preflight: Level 1.
- Terminal rendering of styled HR glyphs: Level 2 via `level2_tree_hr_attributes_render_styled_rule_in_real_terminal`.
- Browser/HTML styled HR output parity: Level 1 string checks only, and only for non-leakage/surrounding text.
- `mark_dim_hr` byte-identical output: not verified.

Because the spec explicitly requires byte-identical output, this is a production-readiness gap even though the focused parser behavior is well covered. Either change the requirement to the weaker current contract, or add assertions that compare the required outputs for the named fixture(s).

### Medium - `BlockExtensionProcessor::warnings()` is collected but unused, with contradictory comments

`BlockExtensionProcessor` collects `StyleWarning`s and exposes a dead-code-allowed `warnings()` accessor. The struct docs say render-tree fold does not surface these and callers should use `scan_inline_hr_warnings`; the unit test comment says the warnings are collected so "Phase 3 can surface it through the fold's diagnostic channel." The fold currently discards the processor after iteration and never reads `warnings()`.

This is not breaking current behavior because `scan_inline_hr_warnings` was rewired to the shared parser, but the comments drift and the unused accessor make the warning contract unclear. Pick one policy:

- If warnings are intentionally preflight-only, remove the unused accessor and update the test/comment to assert parser parity through `scan_inline_hr_warnings`.
- If the fold should surface these warnings, thread them through the fold result or diagnostics and add a test for that public behavior.

## Requirement Coverage

Implemented and covered at Level 1:

- HR-attribute paragraphs are recognized before the fold.
- Bare `---` stays a standard `Event::Rule`.
- Simple-paragraph-only matching is preserved.
- Nested emphasis/code cases do not rewrite.
- Fenced code is not rewritten.
- Blockquote-wrapped HR attributes are rewritten.
- List-item HR-like text is not rewritten.
- Generated HR source span covers the paragraph body, not the paragraph end newline.
- Shared parser handles canonical `kind`, legacy `style`, empty attrs, malformed attrs, and deprecation warnings.

Covered at Level 2:

- Styled HR terminal rendering does not leak raw source and renders through a real terminal pane.

Not sufficiently covered:

- Byte-identical output for existing HR fixtures, especially `mark_dim_hr`.
- Browser/HTML styled HR rendering through a real browser or stronger DOM/style assertion. This may be out of scope if the spec remains parse-side only, but then the byte-identical output language should be narrowed.

## Verdict

Not ready for production as specified. The render-tree parser lift itself is mostly implemented and the focused L1 tests are strong, but the feature does not yet satisfy the spec's architectural retirement criterion or its stated byte-identical parity requirement.

## Verification Run

- `cargo test -p darkmatter --lib block_extension --color=never` - passed, 17 tests.
- `cargo test -p darkmatter render_tree_parity_hr_attributes --color=never` - passed, 2 tests.
- `cargo test -p darkmatter scan_inline_hr_warnings --color=never` - passed, 4 tests.


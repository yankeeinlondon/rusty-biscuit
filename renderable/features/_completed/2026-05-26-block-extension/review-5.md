---
ready: true
agent: codex
model: ""
---

# Review: Block Extension - HR-Attribute Lift, Iteration 5

## Findings

No blocking findings.

The iteration-4 blocker is addressed. The active fold still performs the source-layer inline rewrite before parsing, but `rewrite_inline_extensions` now protects simple HR-attribute paragraph bodies before pairing inline delimiters (`darkmatter/lib/src/markdown/render_tree/inline_extension.rs:313`). That preserves the spec's user-observable block-before-inline behavior for the problematic case: delimiter-like quoted scalars such as `--- { kind: "==waves==" }` reach the block-extension processor as one text paragraph and fold to a generated `ThematicBreak`, not raw paragraph text.

## Requirement Coverage

- HR-attribute paragraph recognition, malformed fallback, legacy `style` warning preflight, blockquote handling, list-item non-rewrite, fenced-code defense, and body-range policy are covered at Level 1 by `block_extension` unit tests.
- The review-4 delimiter-scalar regression is covered at Level 1 in both the source rewriter (`delimiters_inside_hr_attribute_body_are_not_rewritten`) and the fold (`span_aware_fold_hr_attribute_with_delimiter_scalar_still_lifts_to_rule`).
- Source-span parity after an earlier offset-shifting inline rewrite is covered at Level 1 by `span_aware_fold_hr_source_location_survives_earlier_inline_rewrite`.
- Byte-stable tree-pipeline output for `waves`, `kind_waves`, `all_attributes`, and `mark_dim_hr` is covered at Level 1 by `render_tree_hr_snapshots` across HTML, terminal bytes, and MarkdownPlus.
- Terminal user-visible styled HR glyph rendering is covered at Level 2 by `level2_tree_hr_attributes_render_styled_rule_in_real_terminal`, which passed in a real terminal harness.
- No Level 3 coverage is required; this feature has no OS keyboard input behavior.

## Notes

There is a documentation nuance worth keeping in mind for the sibling inline-span work: comments still describe the block-extension processor as sitting between `pulldown-cmark` and inline handling, while the implemented production path is "protect HR bodies during source rewrite, then parse, then block-extension processor." The current tests pin the intended behavior, so this is not a production blocker for this lift.

## Verdict

Ready for production. The HR-attribute lift is decoupled from the removed span-aware HR processor, the shared parser preserves legacy warning behavior, the prior delimiter-scalar leak is covered, and the user-visible terminal rendering requirement has Level 2 verification.

## Verification Run

- `cargo test -p darkmatter --lib block_extension --color=never` - passed, 17 tests.
- `cargo test -p darkmatter --lib span_aware_fold_hr_attribute_with_delimiter_scalar_still_lifts_to_rule --color=never` - passed, 1 test.
- `cargo test -p darkmatter --lib delimiters_inside_hr_attribute_body_are_not_rewritten --color=never` - passed, 1 test.
- `cargo test -p darkmatter --test render_tree_hr_snapshots --color=never` - passed, 3 tests.
- `cargo test -p darkmatter --test level2_render_tree_terminal level2_tree_hr_attributes_render_styled_rule_in_real_terminal --color=never` - passed, 1 test.

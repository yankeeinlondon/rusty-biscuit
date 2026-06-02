---
ready: false
agent: codex
model: ""
---

# Review: Prose to Shared Render Tree, Iteration 3

## Findings

### High: Prose performance gate is still incomplete

The spec makes performance part of the production-readiness contract for this
feature: Prose is called out as a hot primitive, and the migration must add a
Prose render benchmark covering terminal, browser, and markdown over
small/medium/tag-dense corpora, then require no material regression versus the
bespoke emitters (`spec.md:239-252`). The implementation has existing
Criterion coverage in `biscuit-terminal/lib/benches/rendering.rs:44-71`, but it
only benchmarks terminal rendering through `render_optimistic(Some(80))` for
three string shapes. It does not cover browser or Markdown/MarkdownPlus output,
does not include a recorded pre-flip/bespoke baseline, and the feature
directory has no captured middle estimates or accepted-regression note.

This matters because the feature intentionally replaces Prose's private
rendering pipeline with a parse-to-render-tree path plus shared renderer folds.
Even if the functional parity tests are now strong, the spec requires evidence
that this hot path did not materially regress before the feature flips to
production-ready.

Verification level: benchmark evidence, not L1/L2/L3. Current strongest
evidence is an existing terminal-only Criterion benchmark without a bespoke
comparison baseline. That does not satisfy the spec's cross-target performance
gate.

Suggested fix: add or extend a Prose-specific Criterion target that measures
terminal, browser, Markdown, and MarkdownPlus rendering over small, medium, and
tag-dense corpora. Record the old/bespoke baseline or a defensible known
baseline in this feature directory, compare the current tree-only path against
it, and document either neutral-or-better results or the accepted regression
with numbers.

## Closed From Review 2

The prior critical container projection bug is fixed. The new
`fold_prose_nodes_into_blocks` helper preserves block-level `Code` nodes as
siblings while folding contiguous inline runs into paragraphs
(`biscuit-terminal/lib/src/render_tree/projection.rs:327-368`). `BlockQuote`
and list item projection now use that helper for embedded Prose
(`biscuit-terminal/lib/src/components/block_quote.rs:552-570`,
`biscuit-terminal/lib/src/components/list.rs:424-443`).

Coverage for this specific user-observable behavior is now appropriate:
Level 1 tests cover direct tree validity and output for `BlockQuote`,
`OrderedList`, and `UnorderedList`; Level 2 tests exercise `bt quote` and
`bt list` in WezTerm and Kitty and assert visible output plus red SGR selection
(`biscuit-terminal/cli/tests/level2_container_fenced_code.rs:40-202`).

## Verification Performed

- `cargo test -p biscuit-terminal --lib prose_fenced_code_in_block_quote --color=never` passed: 2 tests.
- `cargo test -p biscuit-terminal --lib styled_fenced_code --color=never` passed: 3 tests.

I did not run the Level 2 real-terminal suite in this non-interactive review
pass; I classified its coverage from the checked-in WezTerm/Kitty tests.

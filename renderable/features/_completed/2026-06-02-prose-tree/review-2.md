---
ready: false
agent: codex
model: ""
---

# Review: Prose to Shared Render Tree, Iteration 2

This feature is not ready for production. Iteration 2 fixed the direct Prose styled-fenced-code regression from review 1 and added useful Level 2 coverage for inverse, hidden removal, and styled link fallback behavior. One blocking container projection gap remains.

## Findings

### Critical: Prose fenced code still breaks when Prose is embedded in quote/list containers

The spec says `Prose::to_render_nodes()` remains the embedding API for containers and that code fences, layout, and existing container embedding behavior are preserved. Direct `Prose::render_tree()` handles mixed inline/code content correctly by splitting top-level inline runs into `Paragraph` blocks and leaving `Code` as a block child at `biscuit-terminal/lib/src/components/prose/tree.rs:63`.

The container projections still assume every `Prose` child is inline-only:

- `BlockQuote::project_block_children()` treats `String` and `Prose` as inline content and wraps `paragraph_children()` in one paragraph at `biscuit-terminal/lib/src/components/block_quote.rs:547`.
- `project_list_items()` wraps every projected Prose item in a single paragraph at `biscuit-terminal/lib/src/components/list.rs:430`.

That is invalid once `Prose::to_render_nodes()` can return `NodeKind::Code`. The resulting shape is `Paragraph([..., Code, ...])`, which render-tree validation rejects. I confirmed the user-visible behavior:

- `bt quote $'<red>before\n```\ncode\n```\nafter</red>'` exits with `render tree failed validation with 1 error(s)` from `biscuit-terminal/cli/src/commands/quote.rs:117`.
- `bt quote --md ...` exits with the same validation failure from `biscuit-terminal/cli/src/commands/quote.rs:93`.
- `bt quote --html ...` prints `[render-tree error: render tree failed validation with 1 error(s)]`.
- `bt list ...` and `bt list --md ...` emit empty output because the list renderers swallow `render_terminal_node` / markdown failures into an empty string at `biscuit-terminal/lib/src/components/list.rs:233` and `biscuit-terminal/lib/src/components/list.rs:706`.

Verification level: the direct `bt prose` requirement now has Level 1 and Level 2 coverage, but the embedded-container requirement has no matching Level 1 regression test and no Level 2 real-terminal proof. Because the behavior is currently broken, this is a functionality gap first and a verification gap second.

Suggested fix: add a helper that converts a Prose node sequence into block children by flushing contiguous inline nodes into `Paragraph` and preserving `Code` as a block sibling. Use it in `BlockQuote` and list item projection instead of unconditionally wrapping all Prose nodes in one paragraph. Add Level 1 tests for `BlockQuote`, `OrderedList`, and `UnorderedList` with a styled fenced code Prose child; add Level 2 coverage for at least the CLI quote/list terminal path if this is considered user-facing parity.

## Verification Performed

- `cargo test -p biscuit-terminal --lib code_block_inside_span_restores_enclosing_style --color=never` passed.
- `cargo test -p biscuit-terminal --lib terminal_styled_fenced_code_splits_around_block --color=never` passed.
- `cargo test -p biscuit-terminal --lib test_prose_bold_inline_styling_survives_terminal_tree_render --color=never` passed.
- Manual CLI checks for `bt prose`, `bt quote`, `bt quote --md`, `bt quote --html`, `bt list`, and `bt list --md` with a styled fenced code Prose payload.

I did not run the Level 2 suite in this non-interactive review pass; I classified coverage from the existing test files.

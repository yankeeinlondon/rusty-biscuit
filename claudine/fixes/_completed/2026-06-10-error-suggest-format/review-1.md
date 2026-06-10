---
ready: false
agent: codex
model: ""
---

# Review: StatusBlock Hint Formatting

## Findings

### High: Markdown hints are not italicized

- Requirement: "The hint is italicized in Terminal, MarkdownPlus/Browser where supported, and represented as portable Markdown emphasis when Markdown rendering supports it."
- Implementation: `StatusBlock::to_render_node` marks the hint paragraph with `Style { emphasis.italic = true }` at [status_block.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib/src/components/status_block.rs:276), but the Markdown renderer ignores paragraph-level style and renders `NodeKind::Paragraph` as only `render_inline(children)` at [markdown.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/renderable/src/tree/render/markdown.rs:214). Markdown emphasis is only emitted for semantic `NodeKind::Emphasis` children at [markdown.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/renderable/src/tree/render/markdown.rs:289).
- Impact: `StatusBlock::render_markdown()` puts the hint in the block quote, but the hint remains plain text instead of `_hint_`. That misses an explicit acceptance criterion for portable Markdown output.
- Test gap: `body_plus_hint_renders_hint_inside_block_quote_for_markdown` only asserts that the hint line starts with `>` and contains the hint text at [status_block_parity.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib/tests/status_block_parity.rs:552). It does not assert portable Markdown emphasis.
- Verification level: strongest current coverage is Level 1 string/unit output. Level 1 is appropriate for portable Markdown serialization, but the assertion is incomplete.
- Fix direction: make the hint paragraph contain a semantic `RenderNode::emphasis(vec![RenderNode::text(hint)])` for Markdown-compatible output, or update the Markdown renderer to lower applicable paragraph emphasis. Add a Markdown test that requires `> _Fix this hint_` or the project-standard equivalent.

### High: terminal italic rendering is not verified at the required level

- Requirement: the hint line is italicized when the terminal target supports italics.
- Implementation: the default path carries italic intent in the render tree, and the bespoke path wraps the hint with `<i>...</i>` at [status_block.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib/src/components/status_block.rs:417).
- Test gap: `body_plus_hint_renders_hint_inside_block_quote_for_terminal` strips ANSI before asserting the hint line is block-quoted at [status_block_parity.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib/tests/status_block_parity.rs:528). The custom-border test also strips ANSI. There is no assertion for SGR italic output, and no Level 2 real-terminal capture for whether the styled line renders through a real emulator.
- Verification level: strongest current terminal coverage is Level 1 in-process rendering. For a user-visible terminal styling requirement, this should include at least a Level 1 SGR assertion and a Level 2 capture when marking the feature production-ready under this review's rigor rules.
- Fix direction: add a focused Level 1 assertion that the hint line carries italic SGR on an italic-capable `Terminal`, and add or extend a Level 2 terminal-capture test for a body-plus-hint `StatusBlock` render.

## Coverage Notes

- Body-plus-hint placement inside the block quote is covered for Terminal, Markdown, and Browser at Level 1.
- Blank hints, hint-only blocks, custom-border fallback, custom-border Markdown leakage, `StatusState::Failure`, and every non-deprecated `StatusState` leading blank line are covered at Level 1.
- Claudine-facing regression coverage exists for composition errors with body plus hint via `CompositionError::report_block_error`, also Level 1.

## Verification Run

- `cargo test --color=never -p biscuit-terminal --test status_block_parity` passed.
- `cargo test --color=never -p claudine hint_appears_inside_block_quote_border` passed.

## Production Readiness

Not ready for production. The core block-quote placement behavior is mostly implemented, but the portable Markdown italic requirement is not implemented, and terminal italic styling is not verified strongly enough for the stated review standard.

---
agent: codex
model: ""
ready: true
---

# Review 6

## Findings

No blocking findings.

The prior high-severity gap is addressed. `StatusBlock::to_render_node()` now inserts the structural empty paragraph before a body-attached hint, so the body block quote projects as `blank, body, blank, hint` for the default render-tree path. The newly added parity test also checks the Browser output for four `<p>` elements inside the `<blockquote>`, which closes the Browser-specific part of the previous issue.

The prior documentation drift is also addressed. The public `hint()` docs now describe the body-attached and hint-only behaviors, and the projection docs describe the three output shapes.

## Verification Level Review

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Body starts with a leading blank line inside the block quote | Level 2 tmux capture for `bt status-block`; Level 1 render-tree, terminal, and Markdown assertions | OK |
| Body plus non-blank hint appears inside the same block quote | Level 2 tmux capture; Level 1 Terminal, Markdown, Browser, and Claudine regression assertions | OK |
| Blank separator before body-attached hint | Level 2 tmux capture; Level 1 render-tree, Terminal, Markdown, and Browser structural assertions | OK |
| Hint italic styling | Level 2 WezTerm raw SGR capture; Level 1 Terminal SGR, Markdown emphasis, Browser `<em>`, and render-tree `Emphasis` assertions | OK |
| Blank hints are omitted with no separator | Level 1 render-tree assertion | OK |
| Hint-only output remains outside a block quote | Level 1 render-tree assertion | OK |
| Every current `StatusState` gets the leading blank body layout | Level 1 render-tree assertions, including deprecated `Failure` parity with `Error` | OK |
| Default-border Terminal, Markdown, and Browser use the shared render-tree projection | Level 1 projection parity assertions | OK |
| Custom-border terminal fallback mirrors body/hint layout and italic hint styling | Level 1 bespoke terminal assertions | OK |
| Markdown output does not leak custom terminal border prefixes | Level 1 Markdown assertion | OK |
| Browser output preserves `status-block__hint` class and italic styling | Level 1 Browser assertion | OK |
| Claudine composition errors pick up the improved body-plus-hint layout | Level 1 Claudine rendered-error regressions for unsupported interactive schema, missing properties, and schema load | OK |

No Level 3 coverage is required for this specification because it does not define keyboard, mouse, paste, IME, focus, or terminal input-encoder behavior.

## Tests Reviewed

- `biscuit-terminal/lib/src/components/status_block.rs`
- `biscuit-terminal/lib/tests/status_block_parity.rs`
- `biscuit-terminal/cli/tests/level2_status_block.rs`
- `claudine/lib/src/composition/error.rs`

## Tests Run

- `cargo test --color=never -p biscuit-terminal status_block` passed.
- `cargo test --color=never -p biscuit-terminal --test status_block_parity` passed: 68 tests.
- `cargo test --color=never -p claudine hint_appears_inside_block_quote_border` passed: 3 tests.

I did not run the Level 2 terminal harness in this review; the Level 2 coverage assessment above is from source inspection of `biscuit-terminal/cli/tests/level2_status_block.rs`.

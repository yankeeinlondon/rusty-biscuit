---
agent: codex
model: ""
ready: false
---

# Review 5

## Findings

### High: the render tree still omits the structural blank paragraph before a body-attached hint

The spec makes the render tree contract normative for default-border output and explicitly requires an empty `Paragraph ""` between the body paragraph and `status-block__hint` when a non-blank hint is present ([spec.md:83](spec.md#L83), [spec.md:95](spec.md#L95), [spec.md:101](spec.md#L101)). The implementation only pushes the leading blank paragraph, the body paragraph, and then the hint paragraph directly:

- `block_children.push(RenderNode::paragraph(... String::new() ...))`
- `block_children.push(RenderNode::paragraph(... body_text ...))`
- `block_children.push(hint_node)`

See `biscuit-terminal/lib/src/components/status_block.rs:262-274`.

Terminal and Markdown currently appear to get the desired visual blank row because their block renderers insert spacing between adjacent paragraphs. That is renderer behavior, not the structural empty paragraph required by the spec. Browser output is the more visible miss: `render_html_fragment()` will produce a leading blank paragraph, a body paragraph, and the hint paragraph inside the `<blockquote>`, but no blank paragraph separator before the hint. Existing Browser tests only assert that the hint is inside the block quote and italicized ([status_block_parity.rs:629](../../../biscuit-terminal/lib/tests/status_block_parity.rs#L629), [status_block_parity.rs:804](../../../biscuit-terminal/lib/tests/status_block_parity.rs#L804)); they do not assert the separator node.

Fix: insert a structural empty paragraph immediately before `status-block__hint` in the default projection, and add a render-tree/Browser assertion that the block quote children are `blank, body, blank, hint` for body-plus-hint. The existing terminal and Markdown row-count tests should continue to pass if the renderer handles explicit blank paragraphs correctly.

### Low: public docs still describe `hint()` as below the block quote

The behavior changed so body-attached hints render inside the same block quote, but the builder docs still say `hint()` is "rendered below the block quote" ([status_block.rs:123](../../../biscuit-terminal/lib/src/components/status_block.rs#L123)). The projection docs also say the root has "one optional hint `Paragraph`" alongside the body ([status_block.rs:227](../../../biscuit-terminal/lib/src/components/status_block.rs#L227)), which is now only true for hint-only/no-body cases. This is comment drift in public API documentation.

Fix: update the docs to state that non-blank hints attach inside the body block quote when body content exists, and remain standalone only when there is no body.

## Verification Level Review

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Body starts with a leading blank line inside the block quote | Level 2 terminal capture for `bt status-block` in tmux; Level 1 terminal/Markdown row-count tests | OK for terminal/Markdown; Browser structural separator still covered only partially |
| Body plus non-blank hint appears inside the same block quote | Level 2 tmux capture for visible terminal layout; Level 1 Markdown/Browser assertions | OK |
| Blank separator before body-attached hint | Level 2 tmux and Level 1 Markdown row-count assertions; missing structural/Browser assertion | Gap |
| Hint italic styling | Level 2 WezTerm raw SGR assertion; Level 1 Browser `<em>` and Markdown emphasis assertions | OK |
| Blank hints omitted | Level 1 render-tree assertion | OK |
| Hint-only output remains outside block quote | Level 1 render-tree assertion | OK |
| `StatusState::Failure` matches `Error` | Level 1 render-tree and terminal comparison | OK |
| Custom-border terminal fallback mirrors layout and italic hint | Level 1 terminal assertions | Acceptable; no real-terminal custom-border requirement beyond the shared terminal surface |
| Claudine `UnsupportedInteractiveSchema` regression | Level 1 rendered error assertion | OK for this non-interactive diagnostic; no L2/L3 input behavior is involved |

No Level 3 coverage is required for this spec because it does not define keyboard, mouse, paste, IME, or modifier-key behavior.

## Tests Reviewed

- `biscuit-terminal/lib/tests/status_block_parity.rs`
- `biscuit-terminal/cli/tests/level2_status_block.rs`
- `claudine/lib/src/composition/error.rs` regression tests

I did not run the test suite during this review; the findings above are from source inspection against the spec.

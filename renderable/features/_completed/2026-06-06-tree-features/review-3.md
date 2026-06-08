---
ready: false
agent: codex
model: ""
---

# Review: Tree Features

## Findings

### High: Truncating a styled link or image drops its terminal style reset

The terminal renderer wraps link labels and image placeholders in SGR open and
reset sequences, then passes the complete styled string through
`apply_text_layout`
([render.rs](../../../biscuit-terminal/lib/src/render_tree/render.rs:1018),
[render.rs](../../../biscuit-terminal/lib/src/render_tree/render.rs:1056)).
When truncation is required, `truncate` retains only the visible prefix and
appends the ellipsis
([word_wrap.rs](../../../biscuit-terminal/lib/src/utils/word_wrap.rs:44)).
`split_at_visible_width` places escape sequences encountered after the split in
the discarded tail
([block_constraint.rs](../../../biscuit-terminal/lib/src/utils/block_constraint.rs:216)).
The trailing reset/parent-style restoration is therefore lost.

This is user-visible whenever a width-constrained link or image also has a
foreground, background, or emphasis style: subsequent inline content remains
under the truncated node's terminal style. Both Darkmatter policy paths can
attach color and text-layout hints to the same node
([build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:269),
[build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:305)).

The new image-width tests are unstyled, and the Level 2 truncation tests inspect
plain text only
([render.rs](../../../biscuit-terminal/lib/src/render_tree/render.rs:3120),
[level2_layout.rs](../../../darkmatter/cli/tests/level2_layout.rs:2358),
[level2_layout.rs](../../../darkmatter/cli/tests/level2_layout.rs:2458)).
Shape the visible content before applying the node's SGR envelope, or preserve
and restore active style state when truncating. Add Level 1 raw-output tests for
colored, truncated links and images followed by unstyled text, then add a Level
2 real-terminal test proving the following text does not inherit the color.

## Verification Levels

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Browser and MarkdownPlus alpha lowering | Level 1 plus real-browser computed style | Appropriate |
| Terminal alpha/color degradation | Level 1 plus Level 2 color capture | Gap for styled truncation: reset isolation is broken and untested |
| Link exact/max width and alignment | Level 1 plus Level 2 real-terminal capture | Width is verified; styled truncation is not |
| Image exact/max width and alignment | Level 1 plus Level 2 real-terminal capture | Complete-placeholder width is fixed; styled truncation is not |
| List-item placement | Level 1 plus Level 2 real-terminal capture | Appropriate |
| Structured link/image browser attrs and CSS precedence | Level 1 plus real-browser computed style | Appropriate |
| Root foreground inheritance and frame-only page background | Level 1 structural tests plus real-browser computed style | Appropriate |
| Keyboard, mouse, paste, IME, or hotkey behavior | Not applicable | No Level 3 requirement |

## Verification

- `cargo test -p renderable --color=never`: passed, including 466 unit tests,
  22 integration tests, and 95 doctests.
- Targeted `biscuit-terminal` image text-layout tests: passed.
- Darkmatter tree-feature characterization: 14 passed.
- Darkmatter build-context structural tests: 15 passed.
- Real-browser frame-only translucent-background test: passed.
- `darkmatter/just test-l2`: passed with real harnesses: 15 library tests and
  53 CLI tests.
- `git diff --cached --check`: passed.

The requested `root` skill is not present in the available skill catalog. This
review used the required `renderable` and `rust-testing` skills plus the
repository-root instructions supplied for this session.

## Readiness

Not ready for production. The two iteration-2 findings are fixed and verified,
but width-constrained styled links and images can leak terminal styling into
following content.

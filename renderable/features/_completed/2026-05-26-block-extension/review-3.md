---
ready: true
agent: codex
model: ""
---

# Review: Block Extension - HR-Attribute Lift, Iteration 3

## Findings

### Medium - HR source-span parity is not pinned when an earlier inline rewrite shifts offsets

The implementation now runs `rewrite_inline_extensions(md.content())` before parsing and before `BlockExtensionProcessor` sees paragraph text (`darkmatter/lib/src/markdown/render_tree/fold.rs:418`). That is compatible with the current code because the fold later maps node spans from rewritten offsets back to original offsets (`darkmatter/lib/src/markdown/render_tree/fold.rs:475`), but the HR source-span test only covers an HR at byte `0` with no preceding rewrite (`darkmatter/lib/src/markdown/render_tree/fold.rs:1705`).

The spec requires generated HR source spans to point at the original paragraph body range. There should be a focused Level 1 regression test for a source like:

```markdown
Lead ==marked== text.

--- { style: waves }
```

That test should assert the generated `ThematicBreak` has `Provenance::Generated` and `SourceLocation.bytes` exactly covering the original HR paragraph body, not the rewritten-source range. The existing `mark_dim_hr` snapshots prove rendered bytes, but they do not assert source spans.

## Requirement Coverage

- HR-attribute paragraph recognition, malformed fallback, legacy `style` warning preflight, blockquote handling, list-item non-rewrite, fenced-code defense, and body-range policy are covered at Level 1.
- Byte-stable tree-pipeline output for `waves`, `kind_waves`, `all_attributes`, and `mark_dim_hr` is covered by Level 1 snapshots for HTML, terminal bytes, and MarkdownPlus.
- Terminal user-visible waves rendering is covered at Level 2 by a real terminal pane capture. The previous false-positive risk is addressed by isolating a full waves-glyph rule line.
- No Level 3 coverage is required for this feature; the spec does not require OS keyboard input behavior.

## Verdict

Ready for production. The previous render-side `kind`/`style` gap is fixed, snapshots now show `data-hr-kind` and waves glyph output, and the Level 2 terminal test verifies the styled rule through a real terminal. The remaining item is a useful source-span regression test, not a blocker for the user-visible HR lift.

## Verification Run

- `cargo test -p darkmatter --lib block_extension --color=never` - passed, 17 tests.
- `cargo test -p darkmatter --test render_tree_hr_snapshots --color=never` - passed, 3 tests.
- `cargo test -p darkmatter --test render_tree_parity render_tree_parity_hr_attributes --color=never` - passed, 2 tests.
- `cargo test -p darkmatter --lib scan_inline_hr_warnings --color=never` - passed, 4 tests.
- `cargo test -p darkmatter --test level2_render_tree_terminal level2_tree_hr_attributes_render_styled_rule_in_real_terminal --color=never` - passed, 1 test.

---
ready: true
agent: codex/default
created: 2026-06-30T20:27:34
---

# Review: Unordered List Markers Inside Blockquotes

## Verdict

Ready for production.

I found no blocking defects in the implementation. The change is scoped to `darkmatter/lib/src/markdown/cleanup.rs`, keeps marker extraction unchanged, and makes restoration blockquote-aware while preserving the existing top-level behavior.

## Findings

None.

## Requirement Verification

| Requirement | Strongest verification found | Assessment |
|---|---:|---|
| Blockquoted unordered markers preserve authored `-`, `*`, and `+` markers | Level 1: `cleanup_preserves_dash_marker_for_blockquoted_list_item`, `cleanup_preserves_star_marker_for_blockquoted_list_item`, `cleanup_preserves_plus_marker_for_blockquoted_list_item` in `darkmatter/lib/src/markdown/cleanup.rs` | Sufficient. This is in-process Markdown cleanup behavior, not terminal rendering. |
| CommonMark blockquote prefixes with up to three leading spaces, nested blockquotes, and compact `>>` input preserve authored markers | Level 1: `cleanup_preserves_marker_for_nested_blockquoted_list_item`, `cleanup_preserves_marker_for_compact_blockquoted_list_item`, `cleanup_preserves_marker_for_indented_blockquoted_list_item` | Sufficient. |
| `extract_list_markers` / `restore_list_markers` remain aligned for mixed blockquote and top-level markers | Level 1: `cleanup_preserves_authored_markers_for_mixed_blockquote_and_top_level_lists` | Sufficient. |
| Fenced code content inside blockquotes is protected from marker restoration | Level 1: `restore_list_markers_protects_blockquoted_backtick_fence_content`, `restore_list_markers_protects_blockquoted_tilde_fence_content` | Sufficient. The tests exercise the post-cmark restore surface directly, which is the behavior under review. |
| Existing top-level marker restoration and loose-list behavior do not regress | Level 1: existing `test_loose_list_markers_preserved`; targeted run passed | Sufficient for this cleanup-local change. |

## Notes

- The helper `split_rendered_line` preserves the exact rendered prefix and operates after `fix_blockquote_formatting`, matching the spec's intended ownership boundary.
- No Level 2 or Level 3 tests are required here because the reviewed behavior is Markdown source normalization. There is no terminal-emulator rendering, glyph width, styling, or keyboard-input requirement in this fix.

## Verification Run

Ran:

```text
cargo nextest run --package darkmatter cleanup_preserves_dash_marker_for_blockquoted_list_item cleanup_preserves_authored_markers_for_mixed_blockquote_and_top_level_lists restore_list_markers_protects_blockquoted_backtick_fence_content test_loose_list_markers_preserved --color never
```

Result: 4 passed, 5004 skipped.

---
ready: false
agent: codex
model: ""
---

# Review 4

## Findings

### High: An unmatched policy still changes renderer-wide terminal capabilities

The review-3 regression is not fixed. An unmatched policy makes
`is_default_layout()` false, so `DarkmatterPage::render` sets
`options.max_width = Some(ambient_terminal_width())`
(`darkmatter/lib/src/layout/page.rs:826-832`). The terminal adapter interprets
*any* `Some(max_width)` by constructing `Terminal::new_optimistic`
(`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:533-540`). That changes
the renderer from ambient detection to optimistic capabilities, including
TrueColor, even though the policy matched no node.

The new discriminating Level 1 test demonstrates the defect rather than closing
it: `terminal_unmatched_policy_does_not_flip_color_depth_for_unrelated_content`
fails because the no-policy page emits a plain fenced block while the unmatched
policy page emits TrueColor highlighted output
(`darkmatter/lib/src/layout/page.rs:2007-2072`). Thus policy presence still
changes unrelated code rendering.

The new Level 2 test is a false positive. It asserts that foreground color is
present (`darkmatter/lib/tests/level2_render_tree_terminal.rs:1386-1427`), but
the unintended switch to `Terminal::new_optimistic` is what supplies that
color. It does not compare an unmatched-policy render with the no-policy
baseline, so it cannot prove policy independence.

Keep terminal width and capability selection independent in the adapter rather
than encoding both through `max_width`. Add a Level 1 byte-parity test that
passes under ambient no-color detection and a Level 2 real-terminal parity test
that compares the same unrelated colored content with and without the unmatched
policy.

### High: The color-depth probe adds a second construction fold and an unlisted traversal

`paints_construction_color` calls `to_render_document_with_context` and then
recursively scans the resulting tree (`darkmatter/lib/src/layout/page.rs:1002-1041,
1334-1348`). `render` subsequently calls `render_tree_terminal_with_context`,
which builds the same document again before the target fold
(`darkmatter/lib/src/layout/page.rs:873-889`). Any page with a configured
component/link/image color now performs two complete construction folds plus an
extra tree walk.

This violates the feature's required production topology and acceptance
criterion 3: one complete tree build followed by one target fold. It also makes
the traversal inventory incorrect: the inventory states that
`paints_construction_color` performs no tree traversal
(`renderable/features/2026-06-06-tree-closeout/traversal-inventory.md:63`) and
lists no such probe in the production traversal table.

Capability selection must not require prebuilding and inspecting the document.
Resolve the terminal context independently, or build the `Document` once and
pass that same owned tree to the target renderer. Update the traversal and
performance records after the final design is in place.

## Verification Levels

| Requirement | Strongest present verification | Assessment |
|---|---|---|
| Unmatched policy does not alter unrelated terminal color/capabilities | Level 1 parity test plus Level 2 color-presence test | **Gap.** Level 1 fails; Level 2 asserts the defective optimistic color rather than baseline parity. |
| Page-frame terminal width independence from unmatched policy | Level 1 discriminating parity | Appropriate and passing in the prior iteration. |
| Terminal HR appearance and layout | Level 2 real-terminal capture | Appropriate. |
| Browser HR and component layout/style | Browser computed style/geometry | Appropriate. |
| Markdown/MarkdownPlus degradation | Level 1 | Appropriate. |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement. |

## Focused Verification

- `cargo test -p darkmatter --lib terminal_unmatched_policy_does_not_flip_color_depth_for_unrelated_content --color=never`: **failed**. The no-policy and unmatched-policy outputs differ exactly as described above.
- The Level 2 test was not rerun because its assertion cannot distinguish the regression and the required Level 1 prerequisite is red.

The implementation remains outside the specified architecture and the key
user-visible policy-independence requirement is failing, so this feature is not
ready for production.

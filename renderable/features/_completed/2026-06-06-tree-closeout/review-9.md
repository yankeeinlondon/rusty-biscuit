---
ready: false
agent: codex
model: ""
---

# Review 9

## Findings

### Medium: The parent is marked complete while the closeout remains active and its links are broken

The closeout spec still declares `status: ready for planning and implementation`
and remains under `renderable/features/2026-06-06-tree-closeout`
(`spec.md:2`). However, the plan claims the directory was moved to `_completed`
and all links repaired (`plan.md:315-319`), while the completed CSS Box
Architecture parent links to
`renderable/features/_completed/2026-06-06-tree-closeout/spec.md`
(`../_completed/2026-06-04-css-box-architecture/spec.md:17-26`). That target
does not exist.

This leaves the repository in three contradictory states: the child says it has
not started, its plan says it was completed and moved, and its parent says the
broken-linked child already satisfied every closeout criterion. Acceptance
criterion 9 requires accurate metadata and links, and criterion 11 forbids
marking the parent complete before criteria 1-10 are satisfied.

Either keep the review open by restoring active-path links and removing the
parent's completion claim, or finish the closeout by setting the child metadata
to complete, moving the whole directory to `_completed`, and updating all
references consistently. The latter matches the implemented and verified state.

## Verification Levels

| Requirement | Strongest present verification | Assessment |
|---|---|---|
| Matched layout policy does not alter unrelated color/OSC8 capabilities | Level 1 capability-signature parity | Appropriate regression coverage |
| Same capability parity through a real terminal | Level 2 WezTerm render-in-pane with foreground-color and OSC8 metadata comparison | Appropriate and passing |
| The matched policy actually applies | Level 2 captured table-position difference | Appropriate and non-vacuous |
| Page-frame width independence from unmatched policy | Level 1 discriminating parity | Appropriate |
| Terminal HR appearance and layout | Level 2 real-terminal capture | Appropriate |
| Browser HR and component layout/style | Browser computed style/geometry | Appropriate |
| Markdown/MarkdownPlus degradation | Level 1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Focused Verification

- `cargo test -p darkmatter --test level2_render_tree_terminal level2_render_probe_entrypoint --color=never`: passed.
- `BISCUIT_TEST_LEVEL_REQUIRED=2 just -f darkmatter/justfile test-l2 level2_matched_layout_policy_matches_no_policy_capabilities_in_real_terminal`: passed in WezTerm.
- `cargo metadata --no-deps --format-version 1`: `darkmatter` exposes no probe production binary.
- `git diff --check`: clean.

The iteration-8 traversal-inventory drift is fixed, and the Level 2 capability
test is valid. The feature is not ready until its completion metadata and parent
links describe the repository state accurately.

---
ready: true
agent: codex
model: ""
---

# Review: Tree Features

## Findings

### Low: The policy-attach path still uses clone-modify-store

The spec introduced `*_mut_or_default` specifically so construction-time policy
could populate sparse attrs without clone-modify-store cycles
([spec.md](spec.md:264), [spec.md](spec.md:533)). The helpers exist
([attrs.rs](../../../renderable/src/tree/attrs.rs:1963)), but page, component,
link, and image color application still clone `Style`, mutate the clone, then
box another clone through `set_style`
([build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:171),
[build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:244),
[build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:269),
[build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:305)).

Use `style_mut_or_default` directly and call `retain_non_default_style` after
conditional mutation. This is not a correctness defect, but it leaves the
feature's stated construction ergonomics and allocation reduction incomplete.

The same cleanup should remove or rename the construction-time
`component_for` helper
([build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:214)).
Its behavior is valid and runs beside node construction, but the specification
explicitly names `component_for` among the compatibility paths to delete
([spec.md](spec.md:497)); retaining the old name makes searches for the retired
decorate-time mechanism ambiguous.

### Low: `page_bg_color` documentation contradicts frame-only behavior

`TreeBuildContext::page_bg_color` says the page background is attached to the
root for inheritance
([build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:37)).
The code correctly does the opposite: `apply_page_colors` attaches only the
foreground, while the background remains on the browser/terminal page frame
([build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:162)).

Update the field documentation to describe frame-only paint. Under the
repository's comment rules, the implementation is authoritative and this
behavioral comment drift should not remain.

## Verification Levels

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Browser and MarkdownPlus alpha lowering | Level 1 plus real-browser computed style | Appropriate |
| Terminal alpha/color degradation | Level 1 plus Level 2 color capture | Appropriate |
| Styled link truncation restores following text | Level 1 plus Level 2 real-terminal capture | Appropriate |
| Styled image truncation restores following text | Level 1 plus Level 2 real-terminal capture | Appropriate |
| Link exact/max width and alignment | Level 1 plus Level 2 real-terminal capture | Appropriate |
| Image exact/max width and alignment | Level 1 plus Level 2 real-terminal capture | Appropriate |
| List-item placement | Level 1 plus Level 2 real-terminal capture | Appropriate |
| Structured link/image browser attrs and CSS precedence | Level 1 plus real-browser computed style | Appropriate |
| Root foreground inheritance and frame-only page background | Level 1 structural tests plus real-browser computed style | Appropriate |
| Keyboard, mouse, paste, IME, or hotkey behavior | Not applicable | No Level 3 requirement |

## Verification

- Focused `PaintColor`, typed-attr, and validation suites passed.
- Darkmatter tree-feature characterization passed: 14 tests.
- Darkmatter build-context structural tests passed: 15 tests.
- Focused link/image styled-truncation and trailing-escape tests passed.
- The new image color-reset regression passed in a real WezTerm pane through
  `just _test_l2 darkmatter-cli`: 1 passed, 51 filtered.
- `git diff --check` and `git diff --cached --check` passed.

The requested `root` skill is not present in the authoritative local skill
catalog. This review used `renderable`, `rust-testing`,
`biscuit-test-harness`, and the repository-root instructions.

## Readiness

Ready for production. No high-severity functionality or verification-level
gaps remain. The two findings are non-blocking cleanup that should be handled
to fully realize the stated ergonomics and keep documentation aligned with the
implemented frame-only background contract.

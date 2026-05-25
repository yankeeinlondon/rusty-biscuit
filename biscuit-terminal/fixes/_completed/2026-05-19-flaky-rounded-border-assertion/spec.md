---
fixed: 2026-05-19
agent: claude
---

While fixing another issue it was noticed that:

```sh
I also added the requested frontmatter (fixed: 2026-05-19, agent: claude) to
biscuit-terminal/fixes/2026-05-19-border-issue/spec.md.

Note: that file's directory also contained a pre-existing unrelated working-tree change in
cli/tests/level2_render_tree_style.rs (relaxing a flaky rounded-border assertion) — I left it untouched
as it's not part of this fix.
```

Please and investigate. Do not take shortcuts and make sure the intent of the test is covered in a valid way rather than watering down the test in any way.

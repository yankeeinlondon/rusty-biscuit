---
fixed: 2026-05-19
agent: claude
---

The command `bt hello world --border all` clearly demonstrates that rendering borders is NOT working
correctly at the same time that it demonstrates that our test coverage is weak in this area.

Here is the result of that command:

```sh
💻❯ bt block hello world --border all
┌───────────┐
│ hello world │
└───────────┘
```

The mismatch is visible right there: the top/bottom rules render 2 display columns shorter than the content row, so the corners don't line up with the side bars.

- Top/bottom rule: ┌ + 19 ─ + ┐ = 21 columns
- Content row: │ + + the quick brown fox + + │ = 23 columns

(The awk widths above count bytes — box-drawing glyphs are 3 bytes each — but the column mismatch is the same: 21 vs 23.)

Easiest reproduction: any bt block --border all (or bt quote, which also uses this code path) with content longer than one character.

The longer the content, the more obvious the staircase effect — the right edge ┐/┘ floats 2 columns inside the │ bars.

Root cause in render_border (biscuit-terminal/lib/src/render_tree/style.rs): the content row adds one space of interior padding inside each vertical edge (│ … │), but interior = widest + left + right only accounts for the edge glyphs themselves, not those two padding spaces. So the horizontal rule's run is widest, while the content row's visible width is widest + 2.

- affects square and rounded borders equally, and bt quote
- just masked because the only border test uses single-character content.

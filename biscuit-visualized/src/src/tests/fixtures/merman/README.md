# Merman measurement fixtures

Mermaid sources that the text-measurer tests render through Merman.

- `wrap/`: one diagram per family that wraps labels (flowchart, class,
  state, sequence, ER, mindmap). Each has long labels that must wrap, labels
  with an explicit `<br/>`, and one unbreakable word.
- `mixed/`: flowchart, class, sequence, and mindmap labels in CJK, Arabic,
  Hebrew, emoji, the symbols `→ ✓ ∑ ∞`, precomposed accents, and combining
  marks (e + U+0301, e + U+0308, n + U+0303).

Both sets were written for this repository's text-measurement spike (none
is copied from Merman's or Mermaid's repositories), so no third-party
license applies. They are read with `include_str!` inside each test.

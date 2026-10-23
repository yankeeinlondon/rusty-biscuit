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

- `families/`: one diagram per family of Merman's family registry, for the
  smoke render in `mermaid_family_tests.rs`. Each is the first fixture of
  that family in the comparison spike's corpus (`spikes/s1-comparison/`);
  most are Merman's copies of Mermaid's own examples and specs (file names
  starting `upstream_` there), so `families/LICENSES/` carries Mermaid's MIT
  license and Merman's MIT and Apache-2.0 licenses.

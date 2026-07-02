---
ready: false
agent: codex/default
created: 2026-07-01T06:26:53
implemented: true
---

# Review 1 — Style Everywhere

## Findings

### High — `layout_matrix` does not enforce the required render-path parity

The spec requires `VIA_RENDER == VIA_TREE_DIRECT` for every matrix cell, with
bespoke-path agreement where applicable (`spec.md:530`, `spec.md:641`). The
current harness snapshots both columns but never asserts equality
(`biscuit-terminal/lib/tests/layout_matrix.rs:107`). More importantly, style
scenarios are deliberately applied only to the tree projection:
`render_and_tree` calls `component.render(&term)` first, then merges the
scenario style only onto the projected `RenderNode`
(`biscuit-terminal/lib/tests/layout_matrix_support/mod.rs:373`,
`biscuit-terminal/lib/tests/layout_matrix_support/mod.rs:381`). That means
`background_subtle`, `border_thin_left`, and `emphasis_bold_italic` can pass
while the public `render(&term)` path ignores the user-set style entirely.

Strongest verification present: Level 1 snapshot/unit. The required behavior is
not actually asserted at any level.

### High — Browser/Markdown matrix claims are skipped for terminal-only rows

The matrix marks Browser and Markdown behavior for `HorizontalRule`,
`MetricsTree`, `TerminalImage`, `GraphExpression`, and `MermaidDiagram`
(`matrix.md:47`, `matrix.md:48`, `matrix.md:55`, `matrix.md:60`,
`matrix.md:61`). The Browser/Markdown snapshot tests skip any component case
without `project_tree` (`biscuit-terminal/lib/tests/layout_matrix.rs:121`,
`biscuit-terminal/lib/tests/layout_matrix.rs:140`). Those same components are
registered as `terminal_only!` with `project_tree: None`
(`biscuit-terminal/lib/tests/layout_matrix_support/mod.rs:462`,
`biscuit-terminal/lib/tests/layout_matrix_support/mod.rs:579`,
`biscuit-terminal/lib/tests/layout_matrix_support/mod.rs:597`,
`biscuit-terminal/lib/tests/layout_matrix_support/mod.rs:626`).

This leaves the matrix's Browser-honored / Markdown-degraded cells unverified.
For example, `HorizontalRule` has a browser implementation, but the matrix
harness does not exercise it through the claimed cell coverage.

Strongest verification present: Level 1 terminal-only notes. Required
Browser/Markdown behavior is skipped, so these cells are not production-ready.

### High — Terminal styling is only verified with in-process ANSI checks, not real-terminal capture

The feature asserts user-visible terminal behavior for SGR styling, border
glyphs, background painting, widths, and alignment. The current matrix checks
ANSI substrings and stripped snapshots in process
(`biscuit-terminal/lib/tests/layout_matrix.rs:180`). That is useful Level 1
coverage, but it does not verify a real terminal emulator's cell grid, SGR
decode, glyph widths, or background fill behavior. Under the review rubric,
requirements involving visible glyphs, widths, and SGR styling need Level 2
capture for production confidence.

Strongest verification present: Level 1. Required verification level: Level 2
for user-visible terminal rendering cells involving SGR, glyphs, widths, and
background rectangles.

### High — Darkmatter `style:` parity is not proven against equivalent hand-built render trees

The spec requires a `style:` round-trip suite where each property per component
renders to terminal and HTML matching the equivalent hand-built `renderable`
tree (`spec.md:540`, `spec.md:638`). The new coverage primarily proves schema
reachability and parser/descriptor drift (`darkmatter/lib/src/style/coverage_tests.rs:170`),
with unit tests around lowering. That does not prove that a document using
frontmatter `style:` produces the same terminal/HTML/Markdown result as the
equivalent manually styled render tree for each component and property.

Strongest verification present: Level 1 parser/apply unit coverage. Required
verification level: Level 1 render-output parity, plus Browser/terminal target
checks where the output is user-visible.

## Open Questions

- Should the matrix treat terminal-only/protocol components as truly N/A for
  Browser/Markdown, or should they gain tree/browser fallback projections? The
  current matrix and tests disagree.
- Should the public `render(&term)` path be updated to accept/apply `Style`
  scenarios directly, or should the matrix exclude `render(&term)` from style
  parity? The spec currently requires parity.

## Summary

The implementation expands the schema, matrix, and component tests
substantially, but the core production contract is not closed. The largest
issue is that the test harness can pass while public render paths diverge, and
several claimed matrix cells are skipped entirely.

Production ready: **false**.

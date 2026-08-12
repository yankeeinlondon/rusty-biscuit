---
status: implemented
phase: 6
date: 2026-06-30
---

# Style Everywhere — Property × Component × Target Matrix

> **Spec:** [`spec.md`](spec.md) · **Plan:** [`plan.md`](plan.md)

This matrix records the support contract for every `Layout` / `Style` property on
every renderable component. Each cell is one of:

- **Honored** — the property visibly takes effect, matching the shared fold.
- **Degraded(D1)** — Markdown/MarkdownPlus deliberately drop layout/appearance
  and emit only structural syntax (see Decision D1 in the spec).
- **Degraded(capability)** — a terminal capability fallback (e.g. truecolor → ANSI,
  rounded border → square) is applied.
- **N/A** — the property does not apply to this component kind, with a brief
  rationale.

The matrix is backed by tests:

- Terminal coverage: `biscuit-terminal/lib/tests/layout_matrix.rs` and
  `biscuit-terminal/lib/tests/render_comparison.rs`.
- Per-component parity: `biscuit-terminal/lib/tests/*_parity.rs`.
- Inline-content contract: `biscuit-terminal/lib/tests/inline_content_matrix.rs`.
- Browser/Markdown snapshots: `layout_matrix_browser_snapshots` and
  `layout_matrix_markdown_snapshots` in `layout_matrix.rs`.

## Legend

| Symbol | Meaning |
|--------|---------|
| `H` | Honored |
| `D1` | Degraded — Markdown ignores appearance/layout (structural output only) |
| `Dc` | Degraded — terminal capability fallback |
| `N/A` | Not applicable |

## Block components

| Component | `margin` | `padding` | `width` | `max_width` | `alignment` | `word_wrap` | `color` | `background` | `emphasis` | `border` |
|-----------|----------|-----------|---------|-------------|-------------|-------------|---------|--------------|------------|----------|
| **BlockQuote** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 |
| **Compose** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 |
| **FileSystem** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 |
| **HorizontalRule** | T:H B:H M:D1 | T:N/A B:N/A M:N/A | T:H¹ B:H¹ M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:N/A B:N/A M:N/A | T:N/A² B:N/A² M:N/A² | T:N/A B:N/A M:N/A | T:N/A² B:N/A² M:N/A² | T:N/A B:N/A M:N/A |
| **MetricsTree** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:N/A³ B:N/A³ M:N/A³ | T:H B:H M:D1 | T:N/A³ B:N/A³ M:N/A³ |
| **OrderedList** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H⁴ B:H⁴ M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 |
| **Progress** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:N/A B:N/A M:N/A | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 |
| **Prose** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 |
| **Section** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 |
| **StatusBlock** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H⁴ B:H⁴ M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 |
| **Table** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H⁴ B:H⁴ M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 |
| **TerminalImage** | T:H B:H M:D1 | T:N/A B:N/A M:N/A | T:N/A⁵ B:N/A⁵ M:N/A⁵ | T:N/A⁵ B:N/A⁵ M:N/A⁵ | T:H B:H M:D1 | T:N/A B:N/A M:N/A | T:N/A B:N/A M:N/A | T:N/A B:N/A M:N/A | T:N/A B:N/A M:N/A | T:N/A B:N/A M:N/A |
| **TextBlock** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 |
| **Todo** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H⁴ B:H⁴ M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 |
| **TwoColumn** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H⁴ B:H⁴ M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 |
| **UnorderedList** | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H⁴ B:H⁴ M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 | T:H B:H M:D1 |
| **GraphExpression** | T:H B:H M:D1 | T:N/A B:N/A M:N/A | T:N/A⁶ B:N/A⁶ M:N/A⁶ | T:N/A B:N/A M:N/A | T:H B:H M:D1 | T:N/A B:N/A M:N/A | T:N/A B:N/A M:N/A | T:N/A B:N/A M:N/A | T:N/A B:N/A M:N/A | T:N/A B:N/A M:N/A |
| **MermaidDiagram** | T:H B:H M:D1 | T:N/A B:N/A M:N/A | T:N/A⁵ B:N/A⁵ M:N/A⁵ | T:N/A⁵ B:N/A⁵ M:N/A⁵ | T:H B:H M:D1 | T:N/A B:N/A M:N/A | T:N/A B:N/A M:N/A | T:N/A B:N/A M:N/A | T:N/A B:N/A M:N/A | T:N/A B:N/A M:N/A |

## Inline / badge components

| Component group | `margin` | `padding` | `width` | `max_width` | `alignment` | `color` | `emphasis` | `background` | `border` |
|-----------------|----------|-----------|---------|-------------|-------------|---------|------------|--------------|----------|
| **InlineContent**, **PadLeft**, **PadRight**, **Status** | N/A — inline spans do not own a block box | N/A | N/A | N/A | N/A | T:H B:H M:D1 | T:H B:H M:D1 | T:H⁷ B:H⁷ M:D1 | N/A |

## Darkmatter `style:` policies

| Policy surface | Terminal | Browser | Markdown |
|----------------|----------|---------|----------|
| Every applicable `Layout` / `Style` key per `PageComponent` | H (matches equivalent hand-built `renderable` tree) | H (matches equivalent hand-built `renderable` tree) | D1 (structural Markdown, no styling leakage) |
| Unknown / invalid keys | Rejected by schema canonicalization | Rejected by schema canonicalization | Rejected by schema canonicalization |

## Footnotes

1. `HorizontalRule` honors `Layout::width` on the outer block box and also
   exposes its own `width()` builder (`"20"`, `"50%"`, etc.) that drives the
   glyph/image width. The two contracts are independent and both are honored.
2. `HorizontalRule` uses its own `color()` builder for the glyph/image color;
   `Style::color` / `Style::emphasis` are not lowered onto the irreducible
   glyph/image core.
3. `MetricsTree` delegates to `Prose` for the outer box, but a padding-box
   `background` / `border` would fight the connector glyphs, so they are N/A.
   Per-row coloring is achieved through `Prose` markup (`<red>`, `<dim>`).
4. `word_wrap` for internal-layout text components is honored as a default fed
   into text-bearing cells/items; per-column or per-item policies take
   precedence (Decision D4).
5. `TerminalImage` / `MermaidDiagram` width is controlled by the explicit
   `ImageWidth` contract (`Fill` / `Percent` / `Characters`); `Layout::width`
   and `Layout::max_width` are not read by the image resolver.
6. `GraphExpression` uses `ImageWidth` for the rendered graph canvas;
   `Layout::width` is intentionally N/A to avoid competing width controls.
7. Inline `Span` nodes may honor `Style::background` by painting only the inline
   content, not a padding box.

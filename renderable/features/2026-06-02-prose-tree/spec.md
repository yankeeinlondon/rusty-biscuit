---
status: draft
---

# Prose → Tree: Full Collapse onto the Shared Render Tree

## Status

**Draft — architecture approved.** The core decision is locked: `Prose` stops
carrying its own component-local `ProseDocument` IR and instead parses
**directly into the shared render tree** (`renderable::tree::RenderNode`),
rendering only through the shared Terminal / Browser / Markdown tree renderers.
The bespoke `ProseDocument` emitters are deleted. One small render-side
sub-decision remains — how the browser target lowers `inverse` — recorded under
[Open Questions](#open-questions).

This spec resolves Decision #4 of
[`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md)
("Prose exemption"). It is a holdout migration for that cutover: the migration
itself is cutover **Phase 3**, and its two shared-tree prerequisites are cutover
**Phase 0** fidelity work (they benefit every tree consumer, not just Prose).

Decision lineage (so the choices are not re-litigated):

| Question | Decision | Notes |
|---|---|---|
| Migration degree | **Full collapse.** | Parser emits `RenderNode` directly; `ProseDocument` and the three bespoke emitters are deleted. Cleaner than the projection model and removes a lowering pass. |
| `inverse` (SGR 7) home | **Add to `TextEmphasis`.** | A core SGR attribute, peer to the existing `dim` / `blink`. Not a darkmatter-domain `Extended` token. |
| `hidden` (SGR 8) | **Drop.** | Zero caller uses across the workspace; only the Prose module references it. |
| MarkdownPlus color parity | **Extend the shared markdown renderer.** | Lower an inline `Style` to MarkdownPlus inline-HTML `<span style="…">`, matching today's `to_markdown.rs`. |

## Background

`Prose` is the foundational inline-text primitive: bracketed style tags
(`<red>…</red>`, `<b>`, `<i>`, `<inverse>`), hyperlinks, and a Markdown subset.
It appears in **132 files** across the workspace and is a hot path for CLI
output.

Today `Prose` parses to a private `ProseDocument` IR
(`biscuit-terminal/lib/src/components/prose/ir.rs`) — a pure tree of
`ProseNode` (`Text`, `Span { style, children }`, `Link`, `CodeBlock`) — and
three bespoke emitters render it per target:

- `prose/terminal.rs` — SGR output
- `prose/browser.rs` — HTML
- `prose/to_markdown.rs` — Markdown / MarkdownPlus

`Prose` is the only component still on a component-local IR (the
`component IR (ProseDocument)` state in
[`../../docs/components.md`](../../docs/components.md)). A one-directional
projection already exists: `Prose::to_render_nodes()`
(`prose/tree.rs`) lowers `ProseDocument` into the canonical inline
`RenderNode` shape, and containers such as `BlockQuote` call it to embed
styled prose into the shared tree. That projection is the proven mapping this
spec promotes into the parser itself.

### Why the IR maps cleanly — and the two gaps

`ProseNode` is a structural subset of `RenderNode`. The existing
`prose/tree.rs::node_to_render_node` mapping is:

- `Text` → `NodeKind::Text`
- `Span` (bold/italic/strikethrough) → `Strong` / `Emphasis` / `Delete`
- `Span` (color/bg/dim/blink/underline) → `Span` + a `Style` on `attrs`
- `Link` → `Link` (raw, un-resolved href)
- `CodeBlock` → `Code`

The mapping drops exactly two Prose-only knobs from `ProseStyle`:

1. **`inverse` / `reverse` (SGR 7).** **Load-bearing — 58 uses** in real
   caller code (claudine CLI, darkmatter CLI, darkmatter compose output).
   `renderable::style::TextEmphasis` (`renderable/src/style.rs`) carries
   `bold`, `dim`, `italic`, `strikethrough`, `blink`, `underline` — but **not**
   `inverse`. Dropping it is a visible regression the cutover forbids.
2. **`hidden` (SGR 8).** **Zero uses** outside the Prose module.

### MarkdownPlus color parity gap

`prose/to_markdown.rs` emits MarkdownPlus color with **concrete inline style**:
`<span style="color:#…">text</span>`. The shared markdown renderer's
`render_span` (`renderable/src/tree/render/markdown.rs`) only emits
`<span class="…">` from `attrs.classes`; given a `Style` (no classes) it falls
through to bare text, dropping the color. So routing Prose's Markdown through
the shared renderer regresses MarkdownPlus color unless the shared renderer is
taught to lower an inline `Style` to inline-HTML.

(Plain `Markdown` already degrades color to inner text in both the bespoke and
shared renderers — no gap there. The Terminal and Browser shared renderers
already lower `Style`, so those targets are covered once `inverse` is added.)

## Proposed Architecture

### End state

- `Prose`'s parser builds `RenderNode` directly. `ProseDocument`, `ProseNode`,
  `ProseStyle`, and the three bespoke emitters (`terminal.rs`, `browser.rs`,
  `to_markdown.rs`) are deleted.
- `Prose` renders to every target **only** through the shared tree renderers
  (`render_terminal_node`, `render_browser_*`, `render_markdown_document`).
- `Prose` keeps its bracket-tag parser (`prose/tokens.rs`) — that is its input
  grammar, analogous to darkmatter's Markdown parser, not a rendering path.
- `Prose::to_render_nodes()` becomes the core projection (parse → inline
  `RenderNode`s); containers that already call it are unaffected.

### Shared-tree prerequisites (cutover Phase 0)

These land first, in the `renderable` crate, and benefit every tree consumer:

1. **`inverse` on `TextEmphasis`.** Add `inverse: bool` to
   `renderable::style::TextEmphasis`. Lowering:
   - Terminal → SGR 7 (reverse video), nested with the other emphasis codes.
   - Browser → see [Open Questions](#open-questions).
   - Markdown / MarkdownPlus → no sigil; degrade to inner text.
2. **Inline `Style` → MarkdownPlus HTML.** Extend the shared markdown
   renderer's `render_span` so that, under `MarkdownDialect::MarkdownPlus`, a
   node carrying an inline `Style` (foreground / background color, underline
   variant) emits `<span style="…">` with concrete CSS — matching
   `to_markdown.rs`. Under plain `Markdown` it degrades to inner text (current
   behavior preserved). Class-based spans keep their existing behavior.

### Dropped behavior

- The `<hidden>` / `<reveal>`-style hidden tag is removed from `Prose`. Zero
  callers use it; document the removal in the Prose docs and CHANGELOG-level
  notes. (If a hidden need ever returns, it can be reconsidered as a shared
  attribute then — YAGNI now.)

## Goals

- Remove the last component-local rendering IR so all rendering flows through
  the shared tree (advances cutover Acceptance Criteria #2).
- Preserve Prose fidelity on every target except the two intended diffs
  (dropped `hidden`, browser `inverse` representation).
- Add `inverse` and MarkdownPlus inline-`Style` lowering as first-class shared
  capabilities that darkmatter and other components also gain.
- Keep — or improve — Prose render performance on its hot path.

## Non-Goals

- Changing the Prose bracket-tag **grammar** or adding new tags.
- A general inline-style policy beyond what parity needs.
- Migrating other holdout components (owned by the cutover spec).
- Changing how containers embed Prose (`to_render_nodes()` stays).
- Resolving the broader graphics/image policy (graphics-policy spec).

## Migration Plan

Ordered so each step lands on a green tree:

1. **Add `inverse` to `TextEmphasis`** and its terminal SGR 7 lowering; browser
   and markdown lowering per the decisions above. Unit-test the SGR output and
   the browser/markdown degradation. (Phase 0)
2. **Extend the shared markdown renderer** to lower an inline `Style` to
   MarkdownPlus inline-HTML `<span style>`; plain Markdown unchanged.
   Unit-test against the cases `to_markdown.rs` covers (color, bg, underline).
   (Phase 0)
3. **Pin Prose output** with snapshots of the *current* bespoke terminal /
   browser / markdown / markdown-plus output across a representative tag
   corpus, before any deletion. This is the parity oracle for step 5.
4. **Rewrite the Prose parser to emit `RenderNode`** directly, reusing the
   `node_to_render_node` mapping shapes. `Prose`'s `TerminalRenderable`,
   `BrowserRenderable`, and `MarkdownRenderable` impls delegate to the shared
   renderers over the parsed tree.
5. **Diff against the step-3 snapshots.** The only permitted diffs are the
   removed `<hidden>` and the browser `inverse` representation. Everything else
   must be byte-stable.
6. **Delete** `ir.rs` (`ProseDocument` / `ProseNode` / `ProseStyle`),
   `terminal.rs`, `browser.rs`, and `to_markdown.rs`. Confirm no remaining
   references; `to_render_nodes()` and `prose/tree.rs` fold into the parser
   output path.
7. **Bench Prose** before/after (see [Performance](#performance)).

## Performance

`Prose` is a 132-file hot primitive, so its render cost is tracked explicitly,
separate from the cutover's corpus-wide trend gate.

- Full collapse **removes** the `ProseDocument` allocation and the separate
  `to_render_nodes()` projection pass (parse builds `RenderNode` once).
- It **adds** the shared tree fold over that node list.

Net direction is expected neutral-to-faster, but it must be measured. Add a
Prose render benchmark (terminal + browser + markdown) over a small / medium /
tag-dense corpus and require no material regression versus the bespoke
emitters. If a regression appears, it must clear the cutover's
mild-regression-with-net-faster-trend bar or be fixed before Prose flips.

## Open Questions

Architecture is locked; this is render-side only.

### Browser lowering of `inverse`

SGR 7 swaps foreground and background. The browser has two reasonable
expressions:

- **Swap the resolved fg/bg colors** on the emitted element (semantic, but
  requires both colors to be known at lowering time; defaults may be implicit).
- **`filter: invert(1)`** (or `mix-blend-mode`) — simple and always available,
  but inverts perceptually rather than swapping the two named colors, so it can
  differ from the terminal result.

Recommendation: swap fg/bg when both are resolvable, else fall back to a
documented `filter`-based class. Pin the choice with a browser snapshot. This
is the one intended browser diff versus the bespoke emitter.

## Out of Scope

- Prose grammar changes; new tags.
- Graphics/image policy.
- Other holdout components.
- Public API changes beyond deleting the (already private) `ProseDocument`
  types — `ProseDocument` is not part of Prose's public surface.

## Related Specs

- [`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md) —
  parent cutover; this spec resolves its Decision #4 and supplies its Phase 0
  prerequisites (`inverse`, MarkdownPlus inline-`Style`) and a Phase 3 holdout.
- [`../2026-05-26-inline-span/spec.md`](../2026-05-26-inline-span/spec.md) —
  added `NodeKind::Extended`; considered and rejected as the home for
  `inverse` (a core SGR attribute belongs on `TextEmphasis`, not an extension
  token).
- [`../../docs/components.md`](../../docs/components.md) — component catalog;
  Prose's `IR State` flips from `component IR (ProseDocument)` to
  `tree render only` when this lands.

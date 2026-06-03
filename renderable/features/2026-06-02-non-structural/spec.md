---
status: ready for planning and implementation
reviewed: true
---

# Non-Structural Component Exemptions

## Status

**Architecture approved.** This spec resolves Decision #5 of
[`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md) by
defining the criterion that exempts a component from the cutover's "every
component renders through the tree" requirement, and by producing the
**Exemption Register** — the documented justification Acceptance Criteria #2
requires. It also narrows that criterion's wording.

This is a sub-spec of the tree-cutover work. It has no implementation
dependency on the cutover being complete; instead, the cutover consumes this
spec before Phase 3/5 so those phases know which component holdouts are real
blockers and which are documented non-structural exemptions.

No component in scope here needs to migrate onto the tree for the cutover to
complete. The deliverable is the register plus a small verification condition on
the node-kind renderers.

Decision lineage:

| Question | Decision | Notes |
|---|---|---|
| Exemption criterion | **Document-pipeline participation.** | A component must be `tree render only` iff the darkmatter Markdown→tree document pipeline renders it. Everything else is exempt with documented justification. |
| Component-vs-node distinction | **Node semantics stay on-tree; helper types may stay native.** | `Image`, `ThematicBreak`, and Mermaid code blocks are document nodes. `TerminalImage`, `HorizontalRule`, and `MermaidDiagram` are helper components only when called by the tree renderer. |
| Components in scope | **All exempt** (4 categories). | Terminal-only chrome, standalone graphics/viz widgets, node-kind builder/helpers, and the page frame. |
| Cutover AC#2 wording | **Narrowed.** | "Every component the document pipeline renders is on the tree" — not "every component that exists." |

## Background

The cutover spec's Acceptance Criteria #2 originally read "every renderable
component renders through the tree." Taken literally that is over-broad: it
would force a `RenderNode` projection onto padding primitives, line-concatenation
helpers, status glyphs, and standalone diagram widgets — none of which the
darkmatter document pipeline ever renders, and none of which gain anything from
multi-target tree rendering.

Two kinds of "component" are conflated:

1. **Document-node components.** Their output is a node in a rendered document
   (`BlockQuote`, `Table`, `List`, `Section`, `Code`, images, HR, …). These are
   rendered by the darkmatter Markdown→tree pipeline via a `NodeKind`, and they
   are exactly what the cutover must put on the tree. Most already have
   (`both avail, tree renders`); the rest are tracked by the cutover and its
   sibling specs.
2. **Standalone / presentation components.** Terminal layout primitives, UI
   glyphs, and standalone graphics/viz widgets that an application constructs
   directly. The document pipeline never emits them, so the tree never renders
   them. Forcing a projection adds an abstraction with no consumer.

This spec scopes (2) and exempts it.

## The Criterion

> A component must be `tree render only` **if and only if** the darkmatter
> Markdown→tree document pipeline renders it — i.e. it maps to a
> `renderable::tree::NodeKind`. A component the document pipeline does not render
> is **exempt**, retains its native render path, and does not block bespoke
> deletion. Every exemption is enumerated, with justification, in the
> [Exemption Register](#exemption-register).

The IR's document node kinds (`renderable/src/tree/node.rs`) include `Image`,
`ThematicBreak`, `Code`, `Table`, `List`, `BlockQuote`, `Section`, `Extended`,
etc. There is **no** node kind for graphs, padding, inline-concatenation, status
glyphs, file-dependency trees, or a page frame — which is precisely why those
components are not document content.

Reader note: this criterion applies to **public component types**, not to the
document semantics they might help render internally. A document image, HR, or
Mermaid fence is still a tree node and must be rendered by the corresponding
node renderer. The helper component is exempt only when it is an implementation
detail called from that node renderer.

## Exemption Register

| Component | Crate | Category | Justification |
|---|---|---|---|
| `PadLeft` | biscuit-terminal | terminal layout primitive | Pads content to a minimum width. Terminal-only; no document-structure meaning. |
| `PadRight` | biscuit-terminal | terminal layout primitive | As `PadLeft`. |
| `InlineContent` | biscuit-terminal | terminal layout primitive | Concatenates items on one line with an optional separator. Terminal-only line mechanics. |
| `Status` | biscuit-terminal | terminal UI affordance | A themed status glyph (validation/action state). UI chrome, not document content. |
| `GraphExpression` | biscuit-terminal | standalone graphics widget | Renders a graph from a DSL via `biscuit-visualized`. Multi-target but constructed directly by callers; **not** reached by any darkmatter fence. |
| `FileTree` | darkmatter | standalone viz tool | Visualizes a file's reference/transclusion dependency graph. A CLI/dev tool, terminal-only; not document content. |
| `HorizontalRule` | biscuit-terminal | node-kind builder/helper | Document HRs render via `NodeKind::ThematicBreak` (graphics-policy). The component is a terminal/SVG builder the tree may call as a helper. |
| `TerminalImage` | biscuit-terminal | node-kind builder/helper | Document images render via `NodeKind::Image` (graphics-policy tiers). The component is the image-protocol encoder the tree calls. |
| `MermaidDiagram` | biscuit-terminal | node-kind builder/helper | Document mermaid renders via `NodeKind::Code { lang:"mermaid" }` promotion (graphics-policy). The component is the rasterizer the tree's promotion path will call once implemented; until then the tree renders mermaid as a plain code block. |
| `DarkmatterPage` | darkmatter | page frame / render shell | Wraps document output (margins/padding/background/max-width). Renders to terminal (`render`) and browser (`render_to_browser`), both legacy-backed pre-cutover. The render shell around document output — not a document node. |

### Register Maintenance

Any future `biscuit-terminal` or `darkmatter` component that remains off-tree at
cutover time must be classified before Phase 5:

1. If the darkmatter Markdown→tree document pipeline emits it or relies on it as
   a document node, migrate it to `tree render only`.
2. If it is an internal helper for an existing `NodeKind`, keep the helper
   native but add the node-kind verification described below.
3. If it is terminal-only chrome, standalone application UI, a standalone
   visualization, or a page/render shell, add it to this register with a short
   justification.

Unclassified `no changes` / `both avail, old renders` components are cutover
blockers. They are not silently exempt.

### Categories

- **Terminal layout primitive / UI affordance** (`PadLeft`, `PadRight`,
  `InlineContent`, `Status`) — presentation mechanics with no cross-target
  document meaning. Retain native terminal `render()`. Permanently exempt.
- **Standalone graphics/viz widget** (`GraphExpression`, `FileTree`) — real
  multi-target or terminal renderers, but constructed directly by applications,
  not emitted by the document pipeline. Exempt now; see
  [Optional Future Migration](#optional-future-migration).
- **Node-kind builder/helper** (`HorizontalRule`, `TerminalImage`,
  `MermaidDiagram`) — the document structure they represent *is* on the tree (as
  a node kind); the component is retained as a builder/helper the tree renderer
  may call. Exempt from a *separate* projection. See
  [Verification Condition](#verification-condition).
- **Page frame** (`DarkmatterPage`) — the render shell around document output.

## Verification Condition

The node-kind builder/helper exemptions hold **only if** the tree node renderers
own the document lowering — the darkmatter document pipeline must render
`NodeKind::Image`, `ThematicBreak`, and `Code{lang:"mermaid"}` through the tree
renderers (which may delegate to the exempt component as an internal helper,
e.g. graphics-policy wiring `render_browser_svg` into `render_thematic_break`),
**not** by routing the document through the component's standalone bespoke path.

Before bespoke deletion (cutover Phase 5), confirm for each of `Image`,
`ThematicBreak`, `Code{mermaid}` that the tree renderer produces the document
output self-contained (helper calls are fine) and that removing the legacy
serializers does not orphan it. This is the one place an "exempt builder" could
silently re-introduce a bespoke document path.

Concrete assertions for the Phase 4/5 checklist:

- `NodeKind::ThematicBreak` renders terminal, browser, and markdown output from
  `render_*_node` / `render_*_document`; any `HorizontalRule` call is a pure
  helper call beneath the node renderer.
- `NodeKind::Image` renders through the tree renderer's graphics-policy path;
  `TerminalImage` may encode the chosen terminal tier, but the darkmatter
  document pipeline does not dispatch to `TerminalImage` as a standalone
  component.
- `NodeKind::Code { lang: "mermaid", .. }` stays a code node until the
  Mermaid-aware renderer promotes it according to `MermaidMode` and
  `GraphicsMode`; any `MermaidDiagram` call is below that promotion boundary.
- A mechanical search finds no remaining document-pipeline route that calls the
  legacy `RuleProcessor`, `output/html.rs`, `output/terminal.rs`, or a helper
  component directly for these three node kinds after deletion.

`Status` has a similar naming trap: `StatusBlock` is document content and is on
the tree; bare `Status` is a smaller UI glyph. Before Phase 5, mechanically
confirm no darkmatter document path emits a bare `Status`.

## Refinement to Cutover Acceptance Criteria #2

Proposed wording (the cutover spec is updated to match):

> **Every component the darkmatter document pipeline renders is `tree render
> only`.** Components the document pipeline does not render — terminal-only
> presentation primitives, standalone graphics/viz widgets, node-kind
> builder/helpers, and the page frame — are exempt, enumerated with
> justification in the Non-Structural Component Exemptions spec's Exemption
> Register.

## Optional Future Migration

Exempt does not mean frozen. Revisit a component's exemption when it acquires a
document-pipeline role:

- **`GraphExpression`** — if a `` ```graph `` (or similar) fence is introduced,
  it becomes document content and should lower to a node (likely the same
  `Code`-promotion pattern Mermaid uses, or an `Extended` token). Until then,
  standalone.
- **`FileTree`** — if reference-graph output is ever wanted in browser/markdown
  documents rather than only the CLI, give it a tree projection then.
- **`DarkmatterPage`** — already renders to both terminal and browser
  (legacy-backed); both flip to the tree at cutover Phase 2. If page-frame
  concerns ever need to compose into a document tree, reconsider. Not expected.

These are explicitly **not** cutover blockers.

## Goals

- A single, principled exemption criterion (document-pipeline participation).
- A documented Exemption Register satisfying cutover AC#2's "documented
  justification" clause.
- A verification condition that prevents an "exempt builder" from hiding a
  bespoke document path past Phase 5.

## Non-Goals

- Migrating any exempt component onto the tree (none is required for the
  cutover).
- Designing graph/file-tree fence syntax or node kinds (future, if needed).
- Changing the document node kinds or the components' public APIs.
- The `DarkmatterPage` browser path — it already exists (`render_to_browser`);
  the perf-gate spec only benches it.

## Open Questions

None. The remaining work is verification, not a design decision; the checks are
listed in [Verification Condition](#verification-condition).

## Related Specs

- [`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md) —
  resolves its Decision #5; narrows Acceptance Criteria #2.
- [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md) —
  owns the `Image` / `ThematicBreak` / `Code{mermaid}` node renderers the
  builder/helper exemptions depend on.
- [`../2026-06-02-perf-gate/spec.md`](../2026-06-02-perf-gate/spec.md) —
  benches the existing `DarkmatterPage::render_to_browser`.
- [`../../docs/components.md`](../../docs/components.md) — component catalog and
  IR-state column; exempt components are annotated there.

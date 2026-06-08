---
status: complete (Phase 5)
date: 2026-06-07
owner: ken
spec: renderable/features/2026-06-06-tree-closeout/spec.md
phase: 5
---

# Biscuit Terminal Component Assessment

Assessment of the bespoke biscuit-terminal components named by the
closeout spec (section 8). Each is classified against the four
dispositions:

- **M** — migrate to a canonical multi-target tree projection;
- **H** — add a typed renderer hook / node because the semantic content
  is cross-target;
- **R** — retain as an intentionally terminal-specific primitive;
- **X** — remove or replace if redundant.

This artifact is the durable record for acceptance criterion 10. Phase 1
established the **agreed table shape** and populated the
**production-path column** plus the already-known structural facts (file
location, current `TreeRenderable` / `TerminalRenderable` /
`BrowserRenderable` coverage). Phase 5 completes the **disposition,
target-specific-behavior, blocking-vs-optional, and rationale** columns
now that the extension-hint (`extension-hint-inventory.md`) and
production-traversal (`traversal-inventory.md`) audits are resolved.

## Production-path definition

A component is **on the production path** when it is reachable from the
Darkmatter document render pipeline
(`Markdown -> Document -> target fold`) on Terminal, Browser, Markdown,
or MarkdownPlus. A component constructed only by ad-hoc callers, or a
terminal-only utility the Darkmatter document pipeline never renders,
is **not** on the production path and does not block the parent
cutover.

## Phase 5 outcome (summary)

- **No blocking migrations.** Every component assessed is **R**
  (retained). The three production-path components
  (`HorizontalRule`, `MermaidDiagram`, `TerminalImage`) already carry
  their cross-target semantics on **typed tree nodes** — not on the
  components themselves — so the parent architecture is satisfied
  without flipping any of them. The remaining seven are terminal-only
  utilities or ad-hoc dual-target components that the Darkmatter
  document pipeline never renders.
- **No silent scope expansion.** No production-path migration was
  required, so none was implemented; no new feature specs were created,
  because no candidate migration has concrete user-visible value today
  (each candidate is recorded below as an explicit optional follow-up).
- **Accepted specializations recorded** (see
  [Accepted specializations](#accepted-specializations)): `FileSystem`
  terminal Nerd Font icon selection, `TerminalImage` terminal image
  protocols, and the `MermaidDiagram` terminal rasterization promotion
  policy.

The cross-target semantics for the production-path components live on the
typed tree, verified during this phase:

- `HorizontalRule` ← `NodeKind::ThematicBreak` + typed
  `renderable::tree::ThematicBreakAttrs` (promoted in Phase 2); the
  shared terminal renderer rebuilds the component via
  `horizontal_rule_from_attrs` (`render.rs:585`, `:2015`) and the shared
  browser renderer emits `<hr>`/SVG from the same attrs
  (`render/browser.rs:385`, `:537`).
- `TerminalImage` ← `NodeKind::Image` (built under `GraphicsMode::Rich`
  at `render.rs:1691`, otherwise the `▉ IMAGE[alt]` placeholder at
  `:1055`); browser emits `<img>`, Markdown emits `![alt](url)`.
- `MermaidDiagram` ← `NodeKind::Code` with `lang="mermaid"`, retained
  verbatim on the tree; the terminal renderer promotes it to a raster
  image only under `TerminalMermaidMode::Image` + `GraphicsMode::Rich`
  (`render.rs:1571-1576`), otherwise it stays a code block on every
  target.

## Table (completed)

| Component | Location | Current trait coverage | On production path? | Reachability note | Disposition | Target-specific behavior that cannot be shared | Blocking? | Rationale |
|---|---|---|---|---|---|---|---|---|
| `HorizontalRule` | `biscuit-terminal/lib/src/components/horizontal_rule/mod.rs` (+ `browser.rs`) | `TerminalRenderable` (`mod.rs:152`), `BrowserRenderable` (`browser.rs:221`). No `TreeRenderable`. | **YES** (indirect) | The Darkmatter fold emits `NodeKind::ThematicBreak`; the shared terminal renderer builds an `HorizontalRule` via `horizontal_rule_from_attrs` (`render.rs:2015`), and the shared browser renderer handles it in `render_thematic_break` (`render/browser.rs:537`). Reached through `NodeKind` handling, not attached as a component node. | **R** | Terminal rule glyph/weight/alignment rasterization (`RuleStyle`/`RuleWeight`/`RuleAlignment`) is the terminal lowering of the shared `ThematicBreakAttrs`. | **No** | Cross-target HR semantics already migrated to typed `ThematicBreakAttrs` (Phase 2), shared by Darkmatter HRs and all three renderers. The component is just the terminal build vehicle; no further migration is needed. |
| `GraphExpression` | `biscuit-terminal/lib/src/components/graph_expression.rs` | `TerminalRenderable` (`:372`), `BrowserRenderable` (`:391`). No `TreeRenderable`. | **NO** | Not referenced by the shared terminal renderer or the Darkmatter document pipeline. Constructed by ad-hoc callers wanting a graph/network diagram. | **R** | Terminal ASCII/box graph layout vs browser SVG are independent target lowerings of the same graph model. | **No** | Off the production path and already dual-target via its own traits. A typed graph node with renderer-hook SVG/image lowering (spec disposition **H**) would add Markdown + shared-renderer integration, but has no current user-visible consumer; recorded as an **optional** follow-up, not specced. |
| `MermaidDiagram` | `biscuit-terminal/lib/src/components/mermaid.rs` | `TerminalRenderable` (per `render.rs:44` import and promotion path at `render.rs:1571+`). | **YES** (terminal) | The shared terminal renderer promotes a fenced `mermaid` code block to a rasterized image only under `TerminalMermaidMode::Image` + `GraphicsMode::Rich` (`render.rs:1571-1576`); otherwise the block renders as code. Browser/Markdown render Mermaid as a fenced code block. | **R** | Mermaid → raster image rasterization is a terminal-only protocol concern; browser/Markdown keep the source as a code block. | **No** | The tree already retains the Mermaid source as a `lang="mermaid"` `NodeKind::Code` node and each target chooses its fallback — exactly the spec's question. Terminal rasterization is an accepted target specialization (see below). |
| `TerminalImage` | `biscuit-terminal/lib/src/components/terminal_image/mod.rs` | `TerminalRenderable` (`:227`). No `TreeRenderable`, no `BrowserRenderable`. | **YES** (terminal) | The shared terminal renderer builds a `TerminalImage` for `NodeKind::Image` under `GraphicsMode::Rich` (`render.rs:1691`), falling back to the `[alt]` / `▉ IMAGE[alt]` placeholder (`:1055`). Browser renders `<img>`; Markdown renders `![alt](url)`. | **R** | Kitty/iTerm2/Sixel inline-image protocol encoding and cursor handling are terminal-only and cannot be shared. | **No** | Correctly retained as a terminal protocol primitive *behind* the generic `NodeKind::Image`; the cross-target image semantics live on the typed node. Accepted specialization (terminal image protocols). |
| `Status` | `biscuit-terminal/lib/src/components/status.rs` | `TerminalRenderable` (`:533`). No `TreeRenderable`, no `BrowserRenderable`. | **NO** | Terminal-only inline status indicator. Not referenced by the shared renderers or the Darkmatter document pipeline. (Darkmatter's `StatusBlock` in `markdown/types.rs` is a *different* component.) | **R** | Inline glyph + color status badge is a terminal affordance with no Markdown/Browser document analogue. | **No** | Intentionally terminal-only inline convenience, outside the document pipeline. No cross-target semantic content to share. |
| `MetricsTree` | `biscuit-terminal/lib/src/components/metrics_tree.rs` | `TerminalRenderable` (`:449`). No `TreeRenderable`, no `BrowserRenderable`. | **NO** | Terminal-only structured-metrics tree view. Not referenced by the shared renderers or the Darkmatter document pipeline. | **R** | Box-drawing tree glyphs and aligned numeric columns are a terminal presentation. | **No** | Off the production path. It *could* project to ordinary structured list/table nodes (spec disposition **M**), but it has no cross-target consumer today; recorded as an **optional** follow-up, not specced. |
| `InlineContent` | `biscuit-terminal/lib/src/components/inline_content.rs` | `TerminalRenderable` (`:275`). No `TreeRenderable`, no `BrowserRenderable`. | **NO** | Terminal-only inline-join helper (optional separator). Not referenced by the shared renderers or the Darkmatter document pipeline. | **R** | None unique — it is a thin terminal join convenience. | **No** | Conceptually overlaps structural inline children / `SequenceJoin` on the production path, but it is a terminal-only ergonomic builder for ad-hoc callers with its own tested API. Removal (**X**) is non-blocking and not justified by user-visible value; retained. |
| `PadLeft` | `biscuit-terminal/lib/src/components/pad.rs` | `TerminalRenderable` (`:78`). No `TreeRenderable`, no `BrowserRenderable`. | **NO** | Terminal-only field-formatting utility. Not referenced by the shared renderers or the Darkmatter document pipeline. | **R** | Fixed-width monospace cell padding/alignment is a terminal concern. | **No** | Superseded *on the production path* by typed `TextLayoutHints` (width / alignment / overflow), but retained as a standalone terminal utility for ad-hoc callers. |
| `PadRight` | `biscuit-terminal/lib/src/components/pad.rs` | `TerminalRenderable` (`:179`). No `TreeRenderable`, no `BrowserRenderable`. | **NO** | Terminal-only field-formatting utility. Not referenced by the shared renderers or the Darkmatter document pipeline. | **R** | Fixed-width monospace cell padding/alignment is a terminal concern. | **No** | Same as `PadLeft`: superseded by typed `TextLayoutHints` on the production path; retained as a terminal utility. |
| `FileSystem` | `biscuit-terminal/lib/src/components/filesystem/mod.rs` | `TerminalRenderable` (`:1505`), `TreeRenderable` (`:2803`), `BrowserRenderable` (`:2892`). | **NO** | Has a full tree projection, but the Darkmatter document pipeline never renders it. | **R** | Bespoke Nerd Font terminal icon selection; the target-agnostic projection emits portable Unicode icons the terminal `render` cannot match. | **No** | Already multi-target via its own tree projection; the terminal `render` flip stays **deferred** as an accepted target specialization (Nerd Font icons), not a blocker — `FileSystem` is off the production path. |

## Accepted specializations

These are intentional, recorded terminal specializations — not
architecture gaps. They are accepted because the specialized behavior is
a genuine terminal protocol/affordance with no portable cross-target
equivalent, and (where relevant) the shared semantic content already
lives on the typed tree.

- **`FileSystem` terminal icon selection.** The bespoke terminal
  `render` keeps Nerd Font glyph icons; the shared
  `TreeRenderable`/`BrowserRenderable` projection emits portable Unicode
  icons. The terminal `render` flip remains deferred. `FileSystem` is
  not on the Darkmatter production path, so this does not block the
  parent cutover.
- **`TerminalImage` terminal image protocols.** Inline image rendering
  (Kitty/iTerm2/Sixel encoding, cursor save/restore, scroll
  compensation) is a terminal protocol primitive sitting behind the
  generic `NodeKind::Image`. Browser/Markdown lower the same node to
  `<img>` / `![alt](url)`.
- **`MermaidDiagram` terminal rasterization promotion.** A
  `lang="mermaid"` code node is promoted to a raster image only under
  `TerminalMermaidMode::Image` + `GraphicsMode::Rich`; on every other
  target (and unopted terminals) it stays a fenced code block. The
  Mermaid source is retained verbatim on the tree.

## Optional follow-ups (not specced)

The spec permits creating separate feature specs only for concrete
migrations with user-visible value, and forbids silently expanding
closeout scope. No assessed component met that bar, so no follow-up
specs were created. The two genuine migration candidates are recorded
here for future reference:

- **`GraphExpression` → typed graph node + renderer-hook lowering**
  (disposition **H**). Would add Markdown output and shared-renderer
  integration. Deferred: off the production path, already dual-target,
  no current consumer needing the third target.
- **`MetricsTree` → structured list/table node projection**
  (disposition **M**). Would let it render to Browser/Markdown.
  Deferred: terminal-only today with no cross-target consumer.

If either acquires a user-visible cross-target consumer, open a dedicated
feature spec and link it here.

## Phase 5 exit notes

- All ten listed components (`HorizontalRule`, `GraphExpression`,
  `MermaidDiagram`, `TerminalImage`, `Status`, `MetricsTree`,
  `InlineContent`, `PadLeft`, `PadRight`, `FileSystem`) have a durable
  disposition and one-line rationale — satisfying acceptance criterion 10.
- Every disposition is **R**; **no row is blocking**, so no
  production-path migration was required and none was implemented.
- The three accepted specializations are explicitly recorded, satisfying
  the Phase 5 requirement to record specializations such as `FileSystem`
  terminal icon selection and terminal image protocols.
- No follow-up feature specs were created, in keeping with the spec's
  "do not expand closeout implementation scope silently" rule; the two
  optional migration candidates are documented above instead.

---
status: ready for planning and implementation
reviewed: true
---

# Graphics Policy: Cross-Target Image and Vector Rendering

## Status

**Architecture approved.** This spec defines a single graphics-policy surface
(`GraphicsMode`) on the per-target render contexts, has every graphical
component consult it, brings **Mermaid into scope** as a first-class tree
lowering, and resolves the deferred browser-HR fidelity question in favor of
restoring the styled SVG. A few implementation-level details remain — see
[Open Questions](#open-questions) — but they do not change the architecture.

This spec was carved out of
[`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md) so
the perf spec stays focused on perf and the policy questions — which touch the
shared renderable ↔ biscuit-terminal ↔ darkmatter contract — are reasoned about
as one cross-target concern. It is **Phase 0 of the cutover**
([`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md)):
the terminal `mark_dim_hr` regression and the browser HR fidelity gap both
block the public terminal/browser cutover (DMTR-8), and this work clears them.

Decision lineage (so the choices are not re-litigated):

| Question | Decision | Notes |
|---|---|---|
| `GraphicsMode` variant set | **Three tiers: `Off` / `Vector` / `Rich`.** | With Mermaid in scope the middle tier has genuine weight (static vector vs raster vs code). |
| Middle-tier name | **`Vector`.** | Intent: scalable vector / native-markup graphics — no raster bitmaps, no scripts. |
| Mermaid | **In scope.** | Designed here, not deferred. |
| Mermaid IR shape | **`Code` node, renderer promotes.** | `NodeKind::Code { lang: "mermaid", meta, value }`; the renderer upgrades it to a diagram. Inherits all code-block meta; `Off` degradation is lossless. |
| Browser Mermaid at `Rich` | **Static `<svg>` (default).** | Interactive mermaid.js is an orthogonal browser opt-in, default off. |
| Browser HR fidelity (was B-3 vs B-4) | **B-3 — restore styled `<svg>` at `Vector`+.** | B-4 (ratify the plain-`<hr>` downgrade) rejected; it would violate the cutover's no-regression rule. |
| Default `GraphicsMode` | **`Rich`.** | Behavior-compatible: terminal rasterizes when capable, browser emits styled SVG — legacy parity. |
| Policy placement | **Per-target render context.** | Matches the existing `TerminalRenderContext` / `BrowserRenderContext` split. |
| Bucket A stopgaps | **Dropped.** | This is an implementation spec, not a perf-unblocker; go straight to the framework. |

## Background

"Graphics" in the rusty-biscuit render stack is currently expressed *per
component, per target*, with no shared policy surface:

- **HorizontalRule** — Tier-1 "image" path rasterizes an SVG to PNG and encodes
  it as a Kitty / iTerm2 inline image; Tier-2 falls back to Unicode/ASCII text.
  Browser legacy path emits a CSS-variable-driven `<svg>`; the tree path
  currently emits a plain `<hr data-hr-*>` void tag.
- **TerminalImage** — embeds external images via Kitty / iTerm2 / Sixel /
  half-block fallback. Has no component-level policy enum of its own; darkmatter
  carries `TerminalImageMode` (`Auto` / `Never` / `Force`) at its entry point.
- **Mermaid** — currently deferred on the tree path; legacy darkmatter renders
  `` ```mermaid `` fences via its own `MermaidMode` (`Off` = code block /
  `Image` / `Text`). `MermaidDiagram` (biscuit-terminal, via
  `biscuit-visualized`) rasterizes to PNG for the terminal and has no browser
  path today.

Each component reaches into the terminal/browser context independently and
decides, from raw *capability* signals (`is_tty`, `image_support`), whether to
fire its graphical path. Capability is conflated with policy: there is no shared
"the caller asked us not to render graphics" surface that isn't expressed as a
capability lie.

Two concrete leaks (from
[`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md))
motivate the work:

1. **Terminal `mark_dim_hr` regression.** The tree terminal renderer has no
   equivalent of legacy `TerminalImageMode`; the HR image tier fires off
   `Terminal` capability alone, and the darkmatter tree entry point that maps
   `TerminalOptions → TerminalRenderOptions` **drops `opts.image_mode`
   entirely** (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs`). Net:
   `terminal_no_color` rasterizes 20 PNGs the legacy path skips (≈ 1730× in the
   recorded subset).
2. **Browser HR fidelity downgrade.** The tree browser path emits a plain
   `<hr data-hr-*>` instead of legacy's CSS-variable `<svg>` — a fidelity loss
   tracked as the deferred "HR CSS variables" gap.

Both are graphics-shaped problems wearing perf clothing. The durable fix is a
single graphics-policy surface on the render contexts.

## Conceptual Model

Three axes that are tangled in one decision tree today:

| Axis | Question | Owner |
|------|----------|-------|
| **Capability** | What can the target render? (`image_support`, SVG-in-HTML) | runtime environment (`Terminal`, browser context) |
| **Policy** | What does the caller *want* rendered? | **`GraphicsMode` — this spec** |
| **Fidelity** | If policy permits, how rich a representation? | `capability ∧ policy`, decided per component |

`GraphicsMode` makes **policy** a first-class field on each render context.
Capability stays a property of the environment. Fidelity becomes the product of
`capability ∧ policy`, chosen by each component.

### `GraphicsMode`

```rust
// Cross-target intent names; the per-target context carries the field.
pub enum GraphicsMode {
    /// No graphical lowering. Text/structural fallback only.
    Off,
    /// Scalable vector / native-markup graphics: no raster bitmaps, no
    /// scripts. Browser styled `<svg>`, Mermaid static `<svg>`; terminal has
    /// no vector form, so it degrades to text.
    Vector,
    /// Full fidelity where the target supports it: rasterized images / inline
    /// image protocols (terminal), styled SVG (browser). The default.
    Rich,
}
```

The names describe **intent, not mechanism**; each component maps its own
fidelity ladder onto the tiers, so the *same* output (a vector `<svg>`) can be
the `Vector` tier for one component and a different tier for another. Default is
`Rich` — no behavior change for existing callers.

### Per-component tier mapping

| Component | `Off` | `Vector` | `Rich` |
|-----------|-------|----------|--------|
| HR (terminal) | Unicode / ASCII | Unicode / ASCII | rasterized PNG (image protocol) |
| HR (browser) | plain `<hr>` | styled `<svg>` | styled `<svg>` |
| HR (markdown) | `---` | `---` | `---` |
| Mermaid (terminal) | code block | code block | rasterized PNG |
| Mermaid (browser) | code block | static inline `<svg>` | static inline `<svg>` |
| Mermaid (markdown) | `` ```mermaid `` fence | fence | fence |
| `TerminalImage` (terminal) | alt text | alt text | inline image protocol |

Where a component has no distinct rung for a tier (e.g. HR browser `Vector` vs
`Rich` are both styled SVG; terminal has no vector form) the tiers coincide for
that component. That is expected — not every component uses all three rungs.

This mapping is what fixes both leaks structurally:

- **Terminal raster fires only at `Rich`.** With `TerminalImageMode::Never → Off`
  the no-color / opt-out path stops rasterizing — no capability-field
  overloading.
- **Browser HR SVG is restored at `Vector`+.** Only `Off` yields a plain
  `<hr>`; the default `Rich` matches legacy.

## Proposed Architecture

### B-1: `GraphicsMode` on the render contexts

Add the field to each per-target render context, default `Rich`:

```rust
// renderable::tree — BrowserRenderContext / BrowserRenderOptions
pub struct BrowserRenderContext {
    // existing fields…
    pub graphics_mode: GraphicsMode,
    /// Orthogonal to GraphicsMode. When true, browser Mermaid emits the
    /// client-side mermaid.js path instead of a pre-rendered static <svg>.
    /// Default false. Reproduces legacy `MermaidMode::Image` interactivity.
    pub mermaid_interactive: bool,
}

// biscuit_terminal::render_tree — TerminalRenderContext
pub struct TerminalRenderContext {
    pub terminal: Terminal,            // capability
    pub graphics_mode: GraphicsMode,   // policy
}
```

Markdown has no graphics; its renderer is unaffected.

The darkmatter entry point maps its legacy enums onto the policy:

- `TerminalImageMode::Never → Off`, `Auto → Rich`, `Force → Rich`. `Force`'s
  *capability override* ("attempt regardless of detection") is orthogonal to
  fidelity and stays a separate capability concern, not a `GraphicsMode` value.
- `MermaidMode::Off → code` (the node renders as a code block at any tier below
  promotion), `Image → Rich`, `Text → Off`.
- Where the two legacy enums disagree for a single render (e.g. `image_mode:
  Never` with `mermaid_mode: Image`), a documented precedence rule applies — see
  [Open Questions](#open-questions).

### B-2: Lazy lowering — the renderer picks the tier

The *decision to rasterize* moves out of `HorizontalRule::render` (where it is
the first statement today) to the renderer/context boundary. The component
reports what it *can* render; the renderer, consulting `graphics_mode ∧
capability`, picks the tier. No component fires a rasterization eagerly without
the context's say-so.

### B-3: Restore HR SVG fidelity at `Vector`+

Wire `render_browser_svg`
(`biscuit-terminal/.../horizontal_rule/browser.rs`) into
`render_thematic_break` (`renderable/src/tree/render/browser.rs`) for
`GraphicsMode::Vector` and `Rich`. `Off` keeps a plain `<hr>`. This closes the
deferred "HR CSS variables" gap; the tree browser path reaches parity with
legacy for the styled-HR case. (B-4 — ratifying the downgrade — is rejected.)

### Mermaid as a promoted `Code` node

A `` ```mermaid `` fence already folds to
`NodeKind::Code { lang: "mermaid", meta, value }`. This spec adds **promotion**,
not a new node kind:

- The fence's extended Darkmatter params (`title="…"`, `line-numbering`,
  `highlight=…`) ride on the `Code` node's `meta` and are consumed by the
  `CodeRenderer` hook (`build_code_meta`). Because Mermaid *is* a code node
  until promoted, those params are inherited automatically and the `Off`
  degradation is lossless — a `` ```mermaid `` block at `Off` renders as a full
  titled / line-numbered / highlighted code block.
- Under `Vector`/`Rich` (+ capability), the renderer promotes the node to a
  diagram: terminal rasterizes via `biscuit-visualized` at `Rich`; browser
  emits a pre-rendered static `<svg>` at `Vector`/`Rich`, or the interactive
  mermaid.js path when `mermaid_interactive` is set.
- No `NodeKind::Mermaid` variant is introduced; promotion keys on
  `lang == "mermaid"` + `GraphicsMode`.

### Composition with darkmatter `style:` frontmatter

`GraphicsMode` is the **ceiling**. `style: waves` (a graphical-HR request) is
honored only where the tier allows: styled `<svg>` at `Vector`+ in the browser,
rasterized image at `Rich` on the terminal. At `Off` the structure is honored
(plain `<hr>` / Unicode rule) and the graphic dropped. The `darkmatter.hr.*`
hints stay in the tree unchanged; the renderer gates on policy, so no content is
lost — only its graphical expression is capped.

## Goals

- A single policy (`Off` / `Vector` / `Rich`) by which a caller expresses
  graphics intent **independent of target capability**, applied uniformly to
  every graphical component (HR, `TerminalImage`, Mermaid, future ones) and
  every target with target-appropriate semantics.
- Bring Mermaid onto the tree path as a promoted `Code` node.
- Restore HR SVG fidelity on the tree browser path.
- Stop overloading capability fields (`is_tty`, `image_support`,
  `ColorDepth::None`) to express policy.

## Non-Goals

- Component-level rasterization perf (memoization, SVG-string hygiene,
  `resvg::usvg::Options` reuse) — owned by the perf spec.
- Replacing `resvg` / `tiny_skia` / the rasterization stack.
- Adding *new* graphical components.
- Reworking the legacy `TerminalImageMode` / `MermaidMode` enums; only their
  mapping to `GraphicsMode` at the entry point is in scope.
- Designing the interactive mermaid.js asset/loader story beyond exposing the
  `mermaid_interactive` toggle (its delivery mechanism is a follow-up).

## Migration Plan

Ordered so each step lands on a green tree:

1. **Add `GraphicsMode`** (`renderable::tree`) and the `graphics_mode` field to
   `BrowserRenderContext` and `TerminalRenderContext`, default `Rich`. Update
   every construction site (`from_terminal`, entry points, bench harness).
   Behavior-neutral.
2. **B-2 lazy HR lowering**: move the rasterize decision to the renderer; HR
   honors `graphics_mode ∧ capability`.
3. **B-3**: wire styled `<svg>` into `render_thematic_break` for `Vector`/`Rich`.
4. **Mermaid promotion**: terminal raster at `Rich`; browser static `<svg>` at
   `Vector`/`Rich`; `Off` and no-capability render the code block. Add the
   `mermaid_interactive` browser toggle (default off).
5. **Entry-point mapping**: darkmatter maps `TerminalImageMode` / `MermaidMode`
   → `GraphicsMode`; remove the `image_mode`-dropping bug.
6. **`TerminalImage`**: route its alt-text-vs-image choice through
   `graphics_mode` (`Off`/`Vector` → alt text, `Rich` → image protocol).
7. **Tests + parity**: HR snapshot parity at each tier; `mark_dim_hr`
   no-rasterization at `Off`; Mermaid code-block meta preserved at `Off`;
   browser SVG parity with legacy at `Rich`. Re-run `migration_parity`.

## Open Questions

Architecture is locked; these are implementation-level.

- **Legacy-enum precedence.** When `TerminalImageMode` and `MermaidMode`
  disagree for one render, what is the combined `GraphicsMode`? Likely
  per-component: HR/image honor `TerminalImageMode`, Mermaid honors
  `MermaidMode`, both reading the same `GraphicsMode` ceiling. Pin during
  implementation.
- **`mermaid_interactive` placement / delivery.** Field on
  `BrowserRenderContext` vs a darkmatter page option; how the mermaid.js asset
  is delivered (inline vs CDN). Default off regardless.
- **Does `GraphicsMode` ever belong on `Document`** rather than the per-target
  contexts? Decided: per-target (avoids forcing every caller to reason about
  every target, matches existing option types). Recorded here in case a
  cross-target default surface is later wanted.

## Out of Scope

- Per-component rasterization perf — perf spec.
- Replacing the rasterization stack.
- Interactive mermaid.js asset/loader design beyond the toggle.

## Related Specs

- [`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md) —
  this spec is its Phase 0; clearing both leaks unblocks the public cutover.
- [`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md) —
  the perf spec this carves out from; retains fold-hygiene / no-color items.
- [`../2026-05-20-darkmatter-tree/spec.md`](../2026-05-20-darkmatter-tree/spec.md) —
  parent migration spec.
- [`../2026-05-20-darkmatter-tree/baselines.md`](../2026-05-20-darkmatter-tree/baselines.md) —
  recorded ratios that surface the regressions.
- [`../2026-05-20-darkmatter-tree/entry-point-shape.md`](../2026-05-20-darkmatter-tree/entry-point-shape.md) —
  origin of the deferred "HR CSS variables" gap (now resolved via B-3).

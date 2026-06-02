---
status: ready for planning and implementation
reviewed: true
---

# Graphics Policy: Cross-Target Image and Vector Rendering

## Status

**Architecture approved.** This spec defines a single graphics-policy surface
(`GraphicsMode`) on the per-target render options/context, has every graphical
component consult it, brings **Mermaid into scope** as a first-class tree
lowering through the existing code-renderer extension point, and resolves the
deferred browser-HR fidelity question in favor of restoring the styled SVG.
The remaining open questions are implementation trade-offs that do not change
the architecture.

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
| Mermaid promotion owner | **`CodeRenderer` / adapter-owned promotion.** | `renderable` owns the policy and dispatch point; `darkmatter` / `biscuit-terminal` own Mermaid rendering so `renderable` does not gain a `biscuit-visualized` dependency. |
| Mermaid opt-in | **Legacy `MermaidMode` still controls whether Mermaid is promoted.** | `GraphicsMode` is a ceiling, not a request to promote every `lang="mermaid"` code node. This preserves public defaults where Mermaid fences render as code unless the caller opts into Mermaid rendering. |
| Browser Mermaid at `Rich` | **Static `<svg>` when Mermaid promotion is enabled.** | Interactive mermaid.js is an orthogonal browser opt-in, default off. |
| Browser HR fidelity (was B-3 vs B-4) | **B-3 — restore styled `<svg>` at `Vector`+.** | B-4 (ratify the plain-`<hr>` downgrade) rejected; it would violate the cutover's no-regression rule. |
| HR SVG owner | **Dependency-correct shared builder.** | Do not call from `renderable` into `biscuit-terminal`; move or mirror the pure SVG builder into `renderable` and have `biscuit-terminal` delegate where practical. |
| Default `GraphicsMode` | **`Rich`.** | Behavior-compatible: terminal rasterizes when capable, browser emits styled SVG — legacy parity. |
| Policy placement | **Per-target render options/context.** | `TerminalRenderContext` carries capability + policy; `BrowserRenderOptions` carries browser policy because there is no separate browser context type today. |
| Legacy precedence | **Image mode is the terminal graphics ceiling; Mermaid mode is a component opt-in.** | `TerminalImageMode::Never` suppresses all terminal image attempts, including Mermaid `Image`; `MermaidMode::Off`/`Text` keeps Mermaid as code even when `GraphicsMode::Rich`. |
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
| **Policy** | What does the caller *want* rendered? | **`GraphicsMode` on the render options/context — this spec** |
| **Fidelity** | If policy permits, how rich a representation? | `capability ∧ policy`, decided per component |

`GraphicsMode` makes **policy** a first-class field on each render
options/context. Capability stays a property of the environment. Fidelity
becomes the product of `capability ∧ policy`, chosen by each component.

### `GraphicsMode`

```rust
// Cross-target intent names; the per-target options/context carries the field.
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
The Mermaid rows assume the caller has opted into Mermaid promotion; without
that opt-in, Mermaid remains an ordinary code block at every tier.

This mapping is what fixes both leaks structurally:

- **Terminal raster fires only at `Rich`.** With `TerminalImageMode::Never → Off`
  the no-color / opt-out path stops rasterizing — no capability-field
  overloading.
- **Browser HR SVG is restored at `Vector`+.** Only `Off` yields a plain
  `<hr>`; the default `Rich` matches legacy.

## Proposed Architecture

### B-1: `GraphicsMode` on the render options/context

Add the field to each per-target render options/context, default `Rich`:

```rust
// renderable::tree::render::browser
pub struct BrowserRenderOptions {
    // existing fields…
    pub graphics_mode: GraphicsMode,
    /// Orthogonal to GraphicsMode. Controls whether lang="mermaid" code blocks
    /// remain code, become static SVG, or use the client-side mermaid.js path.
    /// Defaults to Code for public behavior compatibility.
    pub mermaid_mode: BrowserMermaidMode,
}

pub enum BrowserMermaidMode {
    /// Render mermaid fences as ordinary code blocks.
    Code,
    /// Render promoted mermaid fences as sanitized static SVG.
    StaticSvg,
    /// Render promoted mermaid fences through the client-side mermaid.js path.
    Interactive,
}

// biscuit_terminal::render_tree — TerminalRenderContext
pub struct TerminalRenderContext {
    // existing fields…
    pub terminal: Terminal,          // capability snapshot
    pub graphics_mode: GraphicsMode, // policy ceiling
    /// True only for `TerminalImageMode::Force`; attempts image protocol output
    /// even when capability detection says unsupported.
    pub force_graphics: bool,
}
```

Markdown has no graphics; its renderer is unaffected except that Mermaid fences
must remain ordinary fenced code in every markdown dialect.

The darkmatter entry point maps its legacy enums onto the policy and the
component opt-ins:

- `TerminalImageMode::Never → graphics_mode: Off`, `Auto → Rich`, `Force → Rich`
  plus `force_graphics: true`. `Force`'s
  *capability override* ("attempt regardless of detection") is orthogonal to
  fidelity and stays a separate capability concern, not a `GraphicsMode` value.
- `MermaidMode::Off → Code`, `Text → Code`, `Image → Promote`. Promotion is
  still capped by `GraphicsMode`: terminal `image_mode: Never` + `mermaid_mode:
  Image` renders the Mermaid source as a code block and does not attempt `mmdc`,
  `biscuit-visualized`, `resvg`, or image protocol output. This matches the
  legacy terminal contract where `TerminalImageMode::Never` is a deterministic
  image kill switch.

Reader note: this is an intentional split between **ceiling** and **opt-in**.
Using `GraphicsMode::Rich` alone to promote every Mermaid code fence would
change the public defaults of `Markdown::as_html`, `Markdown::for_terminal`,
and the plain tree browser renderer. Keeping Mermaid opt-in separate preserves
those defaults while still giving the tree path a first-class promotion route.

### B-2: Lazy lowering — the renderer picks the tier

The *decision to rasterize* moves out of `HorizontalRule::render` (where it is
the first statement today) to the renderer/context boundary. The component
reports what it *can* render; the renderer, consulting `graphics_mode ∧
capability`, picks the tier. No component fires a rasterization eagerly without
the context's say-so.

### B-3: Restore HR SVG fidelity at `Vector`+

Move the pure styled-HR SVG construction out of
`biscuit-terminal/.../horizontal_rule/browser.rs` into a dependency-correct
helper owned by `renderable` (for example
`renderable::tree::graphics::horizontal_rule_svg`) or mirror it there with
byte-parity tests against the legacy helper during the transition. Then
`render_thematic_break` (`renderable/src/tree/render/browser.rs`) uses that
helper for `GraphicsMode::Vector` and `Rich`; `Off` keeps a plain `<hr>`.

This closes the deferred "HR CSS variables" gap; the tree browser path reaches
parity with legacy for the styled-HR case. It also avoids an accidental
dependency inversion: `renderable` must not depend on `biscuit-terminal` to
render browser HR SVG. Once the shared helper exists, `biscuit-terminal` should
delegate to it where practical so the legacy and tree paths cannot drift. (B-4
— ratifying the downgrade — is rejected.)

### Mermaid as a promoted `Code` node

A `` ```mermaid `` fence already folds to
`NodeKind::Code { lang: "mermaid", meta, value }`. This spec adds **promotion**,
not a new node kind:

- The fence's extended Darkmatter params (`title="…"`, `line-numbering`,
  `highlight=…`) ride on the `Code` node's `meta` and are consumed by the
  `CodeRenderer` hook (`build_code_meta`). Because Mermaid *is* a code node
  until promoted, those params are inherited automatically and the `Off`
  degradation is lossless — a `` ```mermaid `` block with promotion disabled
  or capped by `GraphicsMode::Off` renders as a full titled / line-numbered /
  highlighted code block.
- Promotion is implemented by the target's Mermaid-aware `CodeRenderer` (or an
  equivalent adapter installed through the render options), not by adding a
  `biscuit-visualized` dependency to `renderable`. The renderer dispatch still
  keys on `lang == "mermaid"`, but the package that already owns Mermaid
  rendering performs the actual SVG/PNG generation.
- The code-renderer context must include the effective graphics policy. Terminal
  can carry it through `TerminalCodeContext`; browser should add a small
  `BrowserCodeContext` or install the Mermaid-promoting renderer only when the
  effective browser policy permits promotion. Either way, the hook must be able
  to tell `Off`/`Code` from `Vector`/`Rich` without ambient detection.
- Under `Vector`/`Rich` (+ opt-in + capability), terminal rasterizes via
  `biscuit-visualized` at `Rich`; browser emits a pre-rendered static `<svg>` at
  `Vector`/`Rich`, or the interactive mermaid.js path when browser
  `mermaid_mode` requests it.
- Rendering failure is lossy, not fatal by default: failed Mermaid promotion
  records a diagnostic and falls back to the original code block under
  `RenderStrictness::Warn` / `Lossy`; under `Strict`, the same failure escalates
  according to the existing render-tree strictness model.
- No `NodeKind::Mermaid` variant is introduced.

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
- Preserve existing public defaults: Mermaid fences remain code unless the
  caller opts into Mermaid rendering, and `TerminalImageMode::Never` remains a
  terminal-wide image kill switch.

## Non-Goals

- Component-level rasterization perf (memoization, SVG-string hygiene,
  `resvg::usvg::Options` reuse) — owned by the perf spec.
- Replacing `resvg` / `tiny_skia` / the rasterization stack.
- Adding *new* graphical components.
- Reworking the legacy `TerminalImageMode` / `MermaidMode` enums; only their
  mapping to `GraphicsMode` at the entry point is in scope.
- Designing the interactive mermaid.js asset/loader story beyond exposing the
  `BrowserMermaidMode::Interactive` toggle (its delivery mechanism is a
  follow-up).
- Adding a `biscuit-visualized` or `biscuit-terminal` dependency to
  `renderable`; promotion must stay behind adapters/hooks owned by downstream
  packages.

## Migration Plan

Ordered so each step lands on a green tree:

1. **Add `GraphicsMode`** (`renderable::tree`) and the `graphics_mode` field to
   `BrowserRenderOptions` and `TerminalRenderContext`, default `Rich`. Add the
   separate terminal `force_graphics` capability override and the browser
   Mermaid opt-in field. Update every construction site (`from_terminal`, entry
   points, bench harness). Behavior-neutral for callers that do not opt into
   Mermaid promotion.
2. **B-2 lazy HR lowering**: move the rasterize decision to the renderer; HR
   honors `graphics_mode ∧ capability`.
3. **B-3**: add the dependency-correct HR SVG helper in `renderable`, wire it
   into `render_thematic_break` for `Vector`/`Rich`, and add parity coverage
   against the current `biscuit-terminal` browser HR SVG output.
4. **Mermaid promotion**: implement adapter-owned promotion through the
   Mermaid-aware `CodeRenderer` path. Terminal rasterizes at `Rich`; browser
   emits static `<svg>` at `Vector`/`Rich`; `Off`, no opt-in, no capability, and
   promotion failure render the original code block. Add the browser Mermaid
   opt-in field (default code).
5. **Entry-point mapping**: darkmatter maps `TerminalImageMode` / `MermaidMode`
   → `GraphicsMode`; remove the `image_mode`-dropping bug.
6. **`TerminalImage`**: route its alt-text-vs-image choice through
   `graphics_mode` (`Off`/`Vector` → alt text, `Rich` → image protocol).
7. **Tests + parity**: HR snapshot parity at each tier; `mark_dim_hr`
   no-rasterization at `Off`; `TerminalImageMode::Never` suppresses Mermaid
   `Image`; Mermaid code-block meta preserved when promotion is disabled or
   capped; browser SVG parity with legacy at `Rich`; renderable core does not
   acquire `biscuit-visualized` / `biscuit-terminal` dependencies. Re-run
   `migration_parity`.

## Open Questions

Architecture is locked; these are implementation-level.

- **Interactive Mermaid delivery.** The toggle belongs on browser render
  options, but the mermaid.js asset/loader mechanism still needs a follow-up
  decision:
  - **Inline bundled script.** Pros: deterministic offline output, no CDN
    dependency. Cons: large HTML payload; needs CSP/nonce coordination.
  - **External relative asset.** Pros: works with existing
    `RelativeAssetPath` conventions and keeps HTML smaller. Cons: caller must
    arrange asset copying/serving.
  - **CDN URL.** Pros: smallest HTML and easiest prototype. Cons: network
    dependency and weaker privacy/security defaults.

  Recommendation: use an external relative asset for the production path and
  allow an explicit inline mode for single-file exports. This matches the
  existing browser asset model without making CDN loading the default.
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

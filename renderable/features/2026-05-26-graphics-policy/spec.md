---
status: draft
---

# Graphics Policy: Cross-Target Image and Vector Rendering

## Status

**Draft.** This is an investigation-and-options spec, not an implementation
contract. It carves the graphics/image policy questions out of
`../2026-05-21-isolated-perf/spec.md` so the perf spec can stay focused on
perf, and so the policy questions — which touch the shared
renderable ↔ biscuit-terminal ↔ darkmatter contract — can be reasoned about as
a single cross-target concern rather than per-component, per-target.

No work item here is approved. Buckets exist so a human can decide which (if
any) to schedule before the public terminal/browser cutover (DMTR-8).

## Background

"Graphics" in the rusty-biscuit render stack is currently expressed *per
component, per target*, with no shared policy surface. The relevant
components today are:

- **HorizontalRule** — Tier-1 "image" path rasterizes an SVG to PNG and
  encodes it as a Kitty / iTerm2 inline image; Tier-2 falls back to
  Unicode/ASCII text. Browser legacy path emits a CSS-variable-driven
  `<svg>`; tree path emits a plain `<hr data-hr-*>` void tag.
- **TerminalImage** — embeds external images via Kitty / iTerm2 / Sixel /
  half-block fallback.
- **Mermaid** — currently deferred on the tree path
  (`entrypoints.rs:189-202` docstring: "Image and Mermaid modes stay
  deferred"); legacy darkmatter has its own pipeline.
- **Future graphical components** — anything else that wants to lower to a
  raster or vector representation across targets.

Today each component reaches into the terminal/browser context independently
and decides, from raw *capability* signals (`is_tty`, `image_support`),
whether to fire its graphical path. Capability is conflated with policy:
there is no shared "the caller asked us not to render graphics" surface that
isn't expressed as a capability lie.

Two concrete leaks motivate this spec:

1. **Terminal `mark_dim_hr` regression** —
   `../2026-05-21-isolated-perf/spec.md` Finding 1 — the tree terminal
   renderer has no equivalent of legacy `TerminalImageMode`. The
   `HorizontalRule` image tier fires off `Terminal` capability alone, with
   no way for a caller to opt out. The darkmatter tree entry point that
   maps `TerminalOptions → TerminalRenderOptions` **drops `opts.image_mode`
   entirely** (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs`
   189-202). Net effect: `terminal_no_color` rasterizes 20 PNGs that the
   legacy path skips entirely (≈ 1730× ratio).
2. **Browser HR fidelity downgrade** —
   `../2026-05-21-isolated-perf/spec.md` Finding 2 — the tree browser path
   already emits a plain `<hr data-hr-*>` instead of legacy's
   CSS-variable-driven `<svg>`. This currently *helps* the browser bench
   (less work) but is a **fidelity / parity loss** that was tracked as the
   deferred "HR CSS variables" adapter gap in
   `../2026-05-20-darkmatter-tree/entry-point-shape.md`. The decision is
   owed before the public browser cutover.

Both leaks are graphics-shaped problems wearing perf clothing. Solving them
piecemeal — one ad-hoc fix per component per target — would re-introduce the
same conflation that put them in the perf spec in the first place. The
durable fix is a single graphics policy surface on the render contexts.

## Goals

- Define a **policy** by which a caller expresses graphics intent
  (off / structural / rich) **independent of target capability**.
- Apply the policy uniformly to every component that can lower to a
  rasterized or vector representation: HR, `TerminalImage`, Mermaid,
  future graphical components.
- Apply the policy uniformly across targets (terminal, browser, markdown)
  with target-appropriate semantics — not by inventing a separate field per
  target.
- Resolve the deferred HR CSS-variable / SVG fidelity gap on the tree
  browser path (`../2026-05-20-darkmatter-tree/entry-point-shape.md`).
- Stop overloading capability fields (`is_tty`, `image_support`,
  `ColorDepth::None`) to express policy.

## Non-Goals

- Component-level rasterization optimizations (memoization, SVG-string
  hygiene, `resvg::usvg::Options` reuse). Those live in the perf spec
  (`../2026-05-21-isolated-perf/spec.md`).
- Replacing `resvg` / the rasterization stack.
- Adding new graphical components. The policy must accommodate the existing
  set without prescribing new lowerings.
- Defining a Mermaid lowering for the tree path. The policy must leave a
  door open for one without designing it here.

## Conceptual Model

Three axes that today are tangled in one decision tree:

| Axis | Question | Example today |
|------|----------|---------------|
| **Capability** | What can the target render? | `image_support = Kitty`; `<svg>` is allowed in HTML |
| **Policy** | What does the caller *want* rendered? | `TerminalImageMode::Never`; "structural HR is fine on browser" |
| **Fidelity** | If policy permits, how rich a representation? | image vs Unicode HR; styled `<svg>` HR vs plain `<hr>` |

This spec proposes that policy be a **first-class field on the render
context**, target-shaped but cross-cuttingly named, that every graphical
component consults. Capability stays a property of the runtime environment.
Fidelity becomes the product of `capability ∧ policy`.

### Proposed Vocabulary

A single policy enum, target-replicated:

```rust
// renderable::tree (cross-target naming; concrete enum lives per target
// context type to allow target-specific variants if needed).
pub enum GraphicsMode {
    /// No graphical lowering. HR → Unicode/ASCII; TerminalImage → alt text;
    /// Mermaid → fenced code block or alt text; browser HR → plain `<hr>`.
    Off,
    /// Structural representation only. No raster. Browser may still emit
    /// styled `<svg>` for HR because that is structural at the DOM level;
    /// terminal may emit Unicode/ASCII; Mermaid stays structural.
    Structural,
    /// Full graphical fidelity where the target supports it (the previous
    /// default behavior on image-capable terminals; styled SVG HR in
    /// browser; rasterized Mermaid where wired).
    Rich,
}
```

The names `Off` / `Structural` / `Rich` are deliberately
target-independent — they describe *intent*, not *mechanism*. The
per-target context decides what each variant means concretely.

**Open question:** does `Structural` actually pull its weight, or is the
useful distinction just `Off` vs `Rich`? The HR case wants three states
on browser (no HR, plain `<hr>`, styled `<svg>`) but only two on
terminal (Unicode, image). Investigate before locking the variant set.

## Bucket A — Stopgaps (unblock the perf spec without changing the framework)

Listed first because they unblock the darkmatter no-color cutover without
waiting for the framework decision. None solve the general case.

### A-1: Entry-point capability mutation (was FC-1a)

Have darkmatter's `terminal_options_from_terminal_options` apply the *same*
`Terminal` mutation the legacy path does — under `image_mode: Never` set
`is_tty = false` and `image_support = ImageSupport::None` before building
the `TerminalRenderContext` (`entrypoints.rs:189-202`). No framework type
change; reuses the existing capability guard.

- **Benefit:** removes the ≈ 5.5 ms / ≈ 1730× regression on
  `terminal_no_color` for the darkmatter entry point specifically.
- **Downside:** only callers that route through this darkmatter entry point
  benefit. Any other consumer of the tree terminal renderer (biscuit-terminal
  component users rendering a `HorizontalRule` / image node directly) still
  gets image-eager behavior. It also overloads terminal *capability* fields
  to express a *policy*, which is exactly the conflation this spec exists
  to remove. **Should be removed once Bucket B ships.**

### A-2: Skip HR image tier when `ColorDepth::None` (was B3-1)

Add `if matches!(term.color_depth, ColorDepth::None) { return None; }` to
`render_image_tier`. Cheap, local, removes the no-color rasterization
directly.

- **Trade-off:** changes documented Tier-1 behavior. The current contract
  (darkmatter skill; `mod.rs:288-329`) gates images on `is_tty` +
  `image_support` only. A user who disables ANSI color but still runs an
  image-capable terminal would silently lose graphical rules — conflating
  "no ANSI color" with "no graphics," which are independent axes. **Not
  recommended over A-1**; listed for completeness because it is the
  smallest diff. Same dominance argument as Bucket B applies.

## Bucket B — Framework: Graphics Policy on the Render Context

The durable shape. Adds a graphics-policy field to every render context and
has every graphical component consult it. The capability fields stop
carrying policy.

### B-1: Add `GraphicsMode` to `TerminalRenderContext` and `BrowserRenderContext`

New field on each render context type:

```rust
// renderable::tree::BrowserRenderOptions / BrowserRenderContext
pub struct BrowserRenderContext {
    // existing fields…
    pub graphics_mode: GraphicsMode,
}

// biscuit_terminal::render_tree::TerminalRenderContext
pub struct TerminalRenderContext {
    pub terminal: Terminal,            // capability
    pub graphics_mode: GraphicsMode,   // policy
}
```

Default: `GraphicsMode::Rich` (no behavior change for existing callers that
do not opt in). The darkmatter entry point maps
`TerminalImageMode::Never → Off`, `Auto → Rich`, etc.

**Wiring obligations** for every existing graphical component:

| Component | Off | Structural | Rich |
|-----------|-----|------------|------|
| HR (terminal) | Unicode / ASCII text | Unicode / ASCII text | image tier where supported |
| HR (browser) | omit / `<hr>` plain | `<hr>` plain | styled `<svg>` |
| HR (markdown) | `---` | `---` | `---` *(markdown has no rich form)* |
| `TerminalImage` | alt text | alt text | inline image protocol |
| Mermaid (future) | fenced code | fenced code | rasterized / inline SVG |

**Downsides:**

- `TerminalRenderContext` is shared by **all** tree-rendered components,
  not just darkmatter HRs. Adding a field touches the renderable ↔
  biscuit-terminal contract and every construction site (`from_terminal`,
  the bench harness, all entry points).
- Widens the Stage-3 component-projection surface: every component that can
  rasterize must now honor the policy consistently, or the gap simply moves.

This is the only shape that removes the cliff for *all* tree-terminal
callers, fixes the browser HR fidelity story, and stops the
policy / capability conflation.

### B-2: Lazy / structural HR lowering in the IR (was FC-3)

With B-1 in place, move the *decision to rasterize* from inside
`HorizontalRule::render` (where it lives today as the first statement) to
the renderer / context boundary. The component reports what it *can*
render; the renderer (consulting `GraphicsMode` + capability) picks the
tier.

- **Benefit:** keeps graphical components honest — no component fires a
  rasterization eagerly without the context's say-so.
- **Downside:** changes the `HorizontalRule` component's internal control
  flow and its relationship to the context. Depends on B-1 existing first.

### B-3: Restore HR SVG fidelity on the tree browser path (was B3-3 reverse)

Wire `render_browser_svg` (`biscuit-terminal/.../horizontal_rule/browser.rs`
126-214) into `render_thematic_break`
(`renderable/src/tree/render/browser.rs` 356-371) under
`GraphicsMode::Rich`. Under `Structural` keep the plain
`<hr data-hr-*>`; under `Off` omit the rule entirely or emit a bare `<hr>`
with no styling hints.

- **Benefit:** closes the deferred "HR CSS variables" gap from
  `../2026-05-20-darkmatter-tree/entry-point-shape.md`. Tree browser
  output reaches parity with legacy for the styled-HR case.
- **Cost:** **adds** work to the tree browser path (moves it toward, not
  away from, legacy's number). The current 6× ratio in
  `migration_parity::browser/mark_dim_hr` will worsen for `Rich`. The
  `Structural` setting preserves the current cheap output for callers who
  prefer it.

This is the decision owed before DMTR-8: do we ship structural-only HR on
the public browser path, or do we restore styled SVG? B-1 makes it a
caller-controlled choice instead of a global one.

### B-4: Ratify the tree browser HR downgrade (was B3-3 accept)

The alternative to B-3: document `<hr data-hr-*>` as the canonical tree
browser HR output, drop the `data-*` hints from any external contract, and
record the divergence in the parity ledger.

- **Benefit:** keeps the tree browser path cheap. No new code.
- **Cost:** legacy HTML output (rich themeable SVG) becomes unavailable
  through the tree pipeline. Anyone with a stylesheet hooked into the legacy
  SVG's CSS variables loses it.

**B-3 and B-4 are mutually exclusive.** B-3 needs B-1 to express the
choice cleanly; B-4 needs no framework change but forecloses a feature.

## Bucket C — Ergonomics Trade-Offs (move away from a documented standard)

Listed for completeness; not recommended without a deliberate decision.

### C-1: Make text the default HR tier (was B3-2)

Demote the Tier-1 image path from "primary path for Kitty terminals" to an
opt-in, defaulting all HRs to Unicode/ASCII text.

- **Benefit:** eliminates the rasterization cliff entirely, for every
  terminal — *without* needing graphics policy plumbing at all.
- **Cost:** direct regression of a **shipped, documented darkmatter
  feature** — rich graphical horizontal rules (waves, line-star,
  line-circle, curtain-rod, etc.) rendered as images on Kitty-class
  terminals (darkmatter skill, "Terminal rendering tiers"). The whole
  point of the styled `style:` / `weight:` / `color:` HR attributes is the
  image rendering; demoting it to text strips most of the visual
  distinction between styles.
- **Justification bar:** real-world telemetry showing graphical HRs are
  rarely used. Absent that, this is a feature regression in search of a
  justification.

## Cross-Bucket Summary

| Item | Scope | Removes | Bucket | Note |
|------|-------|---------|--------|------|
| A-1  | darkmatter entry point | terminal no-color cliff (one caller) | Stopgap | Conflates capability/policy; retire after B-1 |
| A-2  | HR image tier | terminal no-color raster | Stopgap | Conflates color/graphics; not recommended |
| B-1  | renderable + biscuit-terminal contract | the conflation | Framework | The durable fix |
| B-2  | `HorizontalRule` internal | eager raster | Framework | Depends on B-1 |
| B-3  | tree browser HR | fidelity gap | Framework | Adds browser cost; needs B-1 |
| B-4  | tree browser HR | (none — accept gap) | Framework | Forecloses SVG HR via tree |
| C-1  | HR Tier-1 default | all raster, all terminals | Ergonomics | Drops a shipped feature |

## Decision Sequencing

1. **Confirm `GraphicsMode` variant set.** Is `Structural` worth its
   weight, or is it `Off` vs `Rich`? Drives B-1's signature.
2. **Decide B-3 vs B-4.** Does the tree browser path ship with styled
   SVG HRs (`Rich`), structural `<hr>` only, or both via policy? This
   blocks the public browser cutover (DMTR-8). If B-3, it must land
   alongside B-1; if B-4, the parity ledger entry can land standalone.
3. **Schedule B-1 + B-2.** B-2 is small once B-1 exists; bundling them
   keeps the `HorizontalRule` lowering change in one patch.
4. **Retire A-1.** Once B-1 ships, the entry-point capability mutation
   should be removed; leaving it in keeps the conflation alive.

## Open Questions

- Does `GraphicsMode` live on each per-target context (`Terminal*` /
  `Browser*`), or as a cross-target field on `Document` / a shared
  `RenderOptions` parent? Per-target avoids forcing every caller to think
  about every target; shared avoids drift between targets.
- How does `GraphicsMode` compose with darkmatter `style:` frontmatter that
  *requests* a graphical HR (`style: waves`)? Does `GraphicsMode::Off`
  suppress the style attribute entirely, or honor structure (`<hr>`) while
  dropping the graphic?
- For Mermaid: should the tree path stub a `Mermaid` node now (rendered as
  a fenced code block under `Off`/`Structural`, deferred for `Rich`), or
  wait for the real lowering? Stubbing makes the policy surface land with
  Mermaid in mind from the start.
- For `TerminalImage`: is the existing `ImageMode`-shaped enum on the
  component itself reconcilable with `GraphicsMode`, or does the component
  retain its own knob inside the broader policy?

## Out of Scope

- Designing a tree-side Mermaid lowering. The policy must accommodate one;
  designing it is a separate spec.
- Per-component rasterization perf (memoization, SVG-string hygiene). Owned
  by `../2026-05-21-isolated-perf/spec.md`.
- Replacing `resvg` / `tiny_skia` / the rasterization stack.
- Reworking the legacy `TerminalImageMode` enum on the darkmatter side. The
  mapping `TerminalImageMode → GraphicsMode` lives at the entry point.

## Related Specs

- `../2026-05-21-isolated-perf/spec.md` — the perf spec this carves out
  from; retains the fold-hygiene and no-color fast-path items.
- `../2026-05-20-darkmatter-tree/spec.md` — the parent migration spec.
- `../2026-05-20-darkmatter-tree/baselines.md` — recorded ratios that
  surface the regressions cited above.
- `../2026-05-20-darkmatter-tree/entry-point-shape.md` — origin of the
  deferred "HR CSS variables" adapter gap (B-3 / B-4 decision).

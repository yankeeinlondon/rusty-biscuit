# Isolated Performance Work: `mark_dim_hr` Tree Regression

## Status

**Draft.** This is an investigation-and-options spec, not an implementation
contract. It dissects the `mark_dim_hr` performance loss surfaced by the
`migration_parity` benchmark suite (see
`../2026-05-20-darkmatter-tree/baselines.md`) and sorts the remediation
candidates into three buckets:

1. **Quick wins** — safe, local, behavior-preserving, no framework or
   ergonomic change.
2. **Framework-level wins** — strong performance benefit, but require a change
   to the shared tree-rendering framework, with downsides to weigh.
3. **Standards / ergonomics trade-offs** — would help performance but move away
   from a documented standard or cost the user something.

No work item here is approved. The buckets exist so a human can decide which
(if any) to schedule before the public terminal/browser cutover (DMTR-8).

## Background

The `mark_dim_hr` fixture exercises darkmatter's custom inline syntax: 80
`==mark==` / `⌄dim⌄` paragraphs and **20 HR-attribute rules**
(`--- { style: …, weight: … }`). It folds through the span-aware processor
chain (`fold_markdown_spanned_with_frontmatter`) so the recorded numbers
include darkmatter's full provenance-preserving path.

Recorded ratios (tree ÷ legacy), from `baselines.md` (2026-05-21):

| Group                       | Fixture       | Tree / Legacy |
|-----------------------------|---------------|---------------|
| `migration/terminal`        | `mark_dim_hr` | ≈ 0.92×       |
| `migration/terminal_no_color` | `mark_dim_hr` | **≈ 1730×**   |
| `migration/browser`         | `mark_dim_hr` | ≈ 6.0×        |
| `migration/markdown`        | `mark_dim_hr` | (tree-only) 578 µs |
| `migration/fold_only`       | `mark_dim_hr` | ≈ 18.2×       |

## Root-Cause Analysis

The "loss" is two distinct, unrelated costs that the original flag conflated.
This spec separates them because they need different fixes.

### Finding 1 — Terminal: HR rasterization is unconditional on the tree path

This is the dominant, real, deterministic regression.

- `Terminal::new_optimistic` advertises a fully image-capable terminal:
  `is_tty = true`, `image_support = ImageSupport::Kitty`
  (`biscuit-terminal/lib/src/terminal.rs:404,414`).
- The tree terminal renderer lowers a `NodeKind::ThematicBreak` by building a
  `HorizontalRule` from its hints and calling `rule.render(&terminal)`
  (`biscuit-terminal/lib/src/render_tree/render.rs:422-424`).
- `HorizontalRule::render` runs the **Tier-1 image path first**
  (`.../horizontal_rule/mod.rs:152-157`). Its only activation guard is
  `is_tty && image_support ∈ {Kitty, ITerm}`
  (`.../horizontal_rule/mod.rs:321-329`). It consults **neither color depth nor
  any image-mode policy.** For each rule it builds an SVG
  (`render_image_svg`), parses it with `resvg::usvg`, rasterizes with
  `tiny_skia`, un-premultiplies alpha, and PNG-encodes the result
  (`rasterize_svg_to_png`, `.../horizontal_rule/mod.rs:698-730`).
- The legacy `for_terminal` path, by contrast, honors `TerminalImageMode`: under
  `Never` it sets `render_terminal.is_tty = false` and
  `image_support = ImageSupport::None`
  (`darkmatter/lib/src/markdown/output/terminal.rs:945-949`). That makes the
  HR image-tier guard fail, so legacy HRs fall to cheap Unicode/ASCII text.
- The darkmatter tree entry point that maps `TerminalOptions →
  TerminalRenderOptions` **drops `opts.image_mode` entirely**
  (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:189-202`; the
  docstring states "Image and Mermaid modes stay deferred").

Net effect: every `mark_dim_hr` render rasterizes 20 PNGs (~275 µs each ≈
5.5 ms) on the tree side, with no way for a caller to turn it off. In the
`terminal_no_color` group, legacy returns in 3.18 µs via its no-color fast path
while the tree path still pays the full 5.50 ms — hence ≈1730×. The same
rasterization cost is present (but masked by the 80 mark/dim paragraphs) in the
TrueColor group, where it nets to ≈0.92×.

### Finding 2 — Browser: the 6× is fold cost, not SVG generation

The baseline note states the browser ratio is "dominated by 20 HR-rule SVG
generations on the tree side." **That is inaccurate.** The tree browser
renderer lowers a `ThematicBreak` to a plain `<hr>` void tag with `data-hr-*`
attributes (`renderable/src/tree/render/browser.rs:356-371`) — it generates **no
SVG**. The rich `<svg>` (CSS-variable driven, `render_browser_svg`,
`.../horizontal_rule/browser.rs:126-214`) is produced only by the **legacy**
HTML path.

So the browser 6× is dominated by the **span-aware fold** (`fold_only` for
`mark_dim_hr` is already 164 µs, ≈18× the plain fold) plus mark/dim inline node
construction — structural costs shared with the markdown target (578 µs). The
HR contribution to the tree browser number is small, and the tree HR browser
output is actually **lower fidelity** than legacy (plain `<hr>` vs styled SVG).
That fidelity gap is a parity item (see Bucket 3), not a perf regression to
optimize away.

### Finding 3 — Fold: span-aware processor overhead

`fold_only` for `mark_dim_hr` is ≈18.2× the legacy parser drain (164 µs vs
9 µs). This is the documented cost of `fold_markdown_spanned_with_frontmatter`
preserving byte ranges and `darkmatter.hr` hints for 100 darkmatter-inline
constructs. It is a real structural cost that every tree target inherits and is
the lower bound for browser/markdown renders of this fixture.

## Bucket 1 — Quick Wins (no risk, no framework or ergonomic change)

These are local, behavior-preserving, and confined to `biscuit-terminal` (or to
allocation hygiene already sanctioned by DMTR-7). None change a public API,
render output, or the tree contract.

### QW-1: Content-addressed rasterization memo for repeated rules

`rasterize_svg_to_png` is pure: identical SVG bytes always produce identical PNG
bytes. A document with many rules sharing the same resolved
style/weight/width/color/cell-size produces identical SVGs. Add a **bounded
thread-local cache** keyed on the SVG bytes (or their hash) inside
`biscuit-terminal`, so a render pass that emits N identical rules rasterizes
once instead of N times.

- **Output:** byte-identical.
- **Scope:** internal to `biscuit-terminal`; no signature or tree change.
- **Cost envelope:** for `mark_dim_hr`, collapses up to 20 rasterizations toward
  the number of *distinct* rule appearances.
- **Only consideration:** a small, explicitly bounded memory footprint (cap the
  cache, e.g. LRU of N entries). It does not change behavior and adds no global
  configuration.

This does not by itself remove the no-color cliff (Finding 1) — identical-input
memoization only helps when rules repeat — so it is complementary to, not a
substitute for, the Bucket 2 image-mode fix.

### QW-2: SVG-string construction micro-optimizations

`render_image_svg` builds the rule SVG with several `format!` calls; the
`Waves` arm grows a `String` with `push_str(&format!(…))` in a loop
(`.../horizontal_rule/mod.rs:418-428`). Pre-size the buffer and write segments
with `write!` into a single `String`. Pure allocation hygiene, behavior- and
byte-preserving. Small benefit; safe.

### QW-3: Reuse `resvg::usvg::Options` instead of constructing per call

`rasterize_svg_to_png` builds `resvg::usvg::Options::default()` on every call
(`.../horizontal_rule/mod.rs:699`). HR SVGs contain no text, so the default
font database is constructed and never used. Investigate constructing it once
(e.g. a `thread_local!` / `OnceLock`-guarded reusable `Options`) and reusing it
across rasterizations. Behavior-preserving if the options are identical to
today's default. (Verify `Options` reuse is sound for the pinned `resvg`
version before adopting.)

### QW-4: DMTR-7 allocation hygiene in the span-aware fold

The 18× `fold_only` ratio (Finding 3) is structural, but a measurement-guided
pass over `fold.rs` / `span.rs` for pre-sizing `SpannedInlineEvent` buffers and
avoiding redundant byte-range / hint-string clones is squarely within the
already-approved DMTR-7 posture. These are local, behavior-preserving cleanups —
not a rewrite. Treat as opportunistic, profiler-led, and bounded.

## Bucket 2 — Framework-Level Wins (strong benefit, framework change required)

These materially fix the regression but require changing the shared
tree-rendering framework. Each lists the change and its downsides to other
parts of the pipeline.

### FC-1: Thread an image / graphics policy into the terminal render context

**This is the fix for the dominant regression (Finding 1).** The tree terminal
renderer has no equivalent of the legacy `TerminalImageMode`. The
`HorizontalRule` image tier (and, later, terminal images and Mermaid) fire
purely off the `Terminal`'s advertised capabilities, with no way for a caller
to say "render structurally / never rasterize."

Two implementation shapes, in increasing invasiveness:

- **FC-1a (narrow, entry-point only):** have darkmatter's
  `terminal_options_from_terminal_options` apply the *same* `Terminal` mutation
  the legacy path does — under `image_mode: Never` set `is_tty = false` and
  `image_support = ImageSupport::None` before building the
  `TerminalRenderContext` (`entrypoints.rs:189-202`). No framework type change;
  it reuses the existing capability guard.
  - **Downside:** only callers that route through this darkmatter entry point
    benefit. Any other consumer of the tree terminal renderer (biscuit-terminal
    component users rendering a `HorizontalRule`/image node directly) still gets
    image-eager behavior. It also overloads terminal *capability* fields to
    express a *policy*, which is exactly the conflation that makes the bug easy
    to reintroduce.

- **FC-1b (framework, correct):** add an explicit graphics/image policy field
  to `TerminalRenderContext` / `TerminalRenderOptions` (e.g. an `ImageMode`
  enum mirroring `TerminalImageMode`) and have the tree renderer consult it
  before invoking any rasterizing tier — HR Tier-1, terminal images, future
  Mermaid. The `HorizontalRule` image tier guard would additionally check this
  policy.
  - **Downside:** `TerminalRenderContext` is shared by **all** tree-rendered
    components, not just darkmatter HRs. Adding a field touches the
    renderable ↔ biscuit-terminal contract and every construction site
    (`from_terminal`, the bench harness, all entry points). It also widens the
    Stage-3 component-projection surface: every component that can rasterize
    must now honor the policy consistently, or the gap simply moves. Larger
    blast radius, but it is the only shape that removes the cliff for *all*
    tree-terminal callers and stops the policy/capability conflation.

**Recommendation to weigh:** FC-1b is the durable fix; FC-1a is a stopgap that
unblocks the darkmatter no-color cutover without solving the general case.
Either way this is the single highest-value change for the recorded numbers
(removes ≈5.5 ms / ≈1730× on `terminal_no_color`).

### FC-2: A no-color fast path for the tree terminal renderer

The spec already pre-commits to this ("`ColorDepth::None` … need explicit fast
paths" — `../2026-05-20-darkmatter-tree/spec.md`, Performance Expectations).
The regression is not HR-specific: `small_prose` no-color is ≈155× (15 µs vs
99 ns) because the tree always builds an owned `Document` and walks it where
legacy short-circuits. A framework-level fast path would let the tree terminal
renderer emit plain text for `ColorDepth::None` (and no graphics) through a
cheaper walk.

- **Benefit:** addresses the no-color regression across *all* fixtures, not just
  `mark_dim_hr`.
- **Downside:** a second terminal-lowering path to keep in sync with the full
  path; risk of behavioral divergence (wrapping, alignment, layout) between the
  two. Must still honor `Layout`/`LineWrapper` semantics or it will silently
  diverge from the full renderer. Meaningful maintenance cost; high value.

This composes with FC-1: with images suppressed (FC-1) *and* a no-color fast
path (FC-2), `mark_dim_hr` no-color drops from 5.50 ms to roughly the fold floor
(~164 µs) plus cheap text assembly.

### FC-3: Lazy / structural HR lowering in the IR

Represent HR rasterization as a deferred capability the target lowers only when
graphics are explicitly enabled, rather than eagerly inside
`HorizontalRule::render`. In practice this overlaps FC-1b (a graphics policy
the renderer checks) and is listed separately only to flag that the cleanest
long-term shape keeps the *decision to rasterize* at the renderer/context
boundary, not buried as the first statement of a component's `render`.

- **Downside:** changes the `HorizontalRule` component's internal control flow
  and its relationship to the context; needs the FC-1b policy to exist first.

## Bucket 3 — Standards / Ergonomics Trade-offs

These would improve performance but move away from a documented standard or cost
the user fidelity/control. Listed so the trade-off is explicit; none are
recommended without a deliberate decision.

### B3-1: Skip the HR image tier when `ColorDepth::None`

Add `if matches!(term.color_depth, ColorDepth::None) { return None; }` to
`render_image_tier`. Cheap, local, and removes the no-color rasterization
directly.

- **Trade-off / loss:** changes documented behavior. The Tier-1 contract
  (darkmatter skill; `mod.rs:288-329`) gates images on `is_tty` +
  `image_support` *only*. A user who disables ANSI color (`--no-color`) but
  still runs an image-capable terminal would silently lose graphical rules —
  conflating "no ANSI color" with "no graphics," which are independent axes. It
  also still leaves images firing for any non-`None` color depth, so it is a
  partial fix that muddies the capability model. FC-1 expresses the same intent
  without the conflation.

### B3-2: Make text the default HR tier; rasterize only on explicit opt-in

Demote the Tier-1 image path from "primary path for Kitty terminals" to an
opt-in, defaulting all HRs to Unicode/ASCII text.

- **Benefit:** eliminates the rasterization cliff entirely, for every terminal.
- **Trade-off / loss:** this is a direct regression of a **shipped, documented
  darkmatter feature** — rich graphical horizontal rules (waves, line-star,
  line-circle, curtain-rod, etc.) rendered as images on Kitty-class terminals
  (darkmatter skill, "Terminal rendering tiers"). The whole point of the styled
  `style:`/`weight:`/`color:` HR attributes is the image rendering; demoting it
  to text strips most of the visual distinction between styles. A significant
  ergonomic/feature loss; only justified if image HRs prove not worth their
  cost in real use.

### B3-3: Ratify (or reverse) the tree browser HR fidelity downgrade

Per Finding 2, the tree browser path already emits a plain `<hr data-hr-*>`
instead of legacy's CSS-variable-driven `<svg>`. This currently *helps* the
browser benchmark (less work) but is a **fidelity/parity loss**: the rich,
themeable SVG rule is replaced by a bare rule with hint data attributes. This
was tracked as the deferred "HR CSS variables" adapter gap in
`../2026-05-20-darkmatter-tree/entry-point-shape.md`.

A decision is owed before the public browser cutover (DMTR-8):

- **Accept the downgrade** (ratify plain `<hr>` + `data-*`, document the
  divergence in the parity ledger) — keeps the tree browser path cheap but
  diverges from legacy visual output.
- **Restore parity** by wiring `render_browser_svg` into `render_thematic_break`
  — recovers fidelity but **adds** cost to the tree browser path (moving it
  toward, not away from, legacy's number). This is a fidelity item, not a perf
  win, which is why it sits in Bucket 3 rather than Bucket 2.

## Cross-Bucket Summary

| Item  | Target            | Removes          | Bucket | Note |
|-------|-------------------|------------------|--------|------|
| QW-1  | Terminal          | duplicate raster | 1 | Helps only when rules repeat |
| QW-2  | Terminal          | SVG-string allocs| 1 | Micro |
| QW-3  | Terminal          | per-call `Options`| 1 | Verify reuse sound |
| QW-4  | Fold (all targets)| fold allocs      | 1 | DMTR-7 hygiene, profiler-led |
| FC-1  | Terminal          | **the cliff**    | 2 | Highest value; 1a stopgap vs 1b durable |
| FC-2  | Terminal          | no-color overhead| 2 | All fixtures, not just HR |
| FC-3  | Terminal (IR)     | eager raster     | 2 | Overlaps FC-1b |
| B3-1  | Terminal          | no-color raster  | 3 | Conflates color/graphics |
| B3-2  | Terminal          | all raster       | 3 | Drops a shipped feature |
| B3-3  | Browser           | (fidelity)       | 3 | Decision owed for cutover |

## Open Questions

- Should the graphics policy (FC-1b) be a distinct `ImageMode` on the render
  context, or should the tree renderer derive it from existing terminal
  capability fields the way components do today?
- Is QW-1's repeated-rule memoization worth its bounded cache given that FC-1
  already removes the dominant no-color cost? (It still helps colored,
  image-capable terminals that render many rules.)
- Does the public browser cutover require restoring HR SVG fidelity (B3-3
  reverse), or is the `<hr data-*>` representation acceptable with a documented
  ledger entry?
- Before scheduling FC-2, what is the standard memory profiler for this
  workspace (still an open question from DMTR-6)?

## Out of Scope

- Aggressive tuning the parent migration explicitly defers (arenas, interning,
  `Arc<str>`, lifetime-parametric nodes, broad `SmallVec`).
- Replacing `resvg` / the rasterization stack.
- Any change to `pulldown-cmark` parsing or the span-aware provenance contract
  beyond local allocation hygiene (QW-4).

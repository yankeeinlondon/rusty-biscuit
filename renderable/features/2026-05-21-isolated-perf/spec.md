# Isolated Performance Work: Tree Pipeline Hotspots

## Status

**Draft.** This is an investigation-and-options spec, not an implementation
contract. It captures the perf regressions the `migration_parity` benchmark
suite surfaced (see `../2026-05-20-darkmatter-tree/baselines.md`) and sorts
the remediation candidates into two buckets:

1. **Quick wins** — safe, local, behavior-preserving, no framework or
   ergonomic change.
2. **Framework-level wins** — strong perf benefit but require a change to
   the shared tree-rendering framework.

The graphics / image policy questions that were originally part of this
spec have been carved out into
`../2026-05-26-graphics-policy/spec.md`. That spec owns FC-1 (graphics
policy on the render context), FC-3 (lazy HR lowering), the B3-1 / B3-2
ergonomics trade-offs around HR image tiers, and the B3-3 tree browser HR
fidelity decision. They are policy / architecture questions, not perf
questions, and reasoning about them per-component would re-introduce the
same conflations the perf numbers exposed.

What remains here is the *behavior-preserving* perf work that survives
regardless of how the graphics-policy spec lands.

No work item here is approved. The buckets exist so a human can decide
which (if any) to schedule alongside the public terminal / browser cutover
(DMTR-8).

## Background

Recorded ratios (tree ÷ legacy) from `baselines.md` (2026-05-21) for the
`mark_dim_hr` fixture — 80 `==mark==` / `⌄dim⌄` paragraphs and 20
HR-attribute rules — routed through the span-aware processor chain
(`fold_markdown_spanned_with_frontmatter`):

| Group                         | Fixture       | Tree / Legacy        |
|-------------------------------|---------------|----------------------|
| `migration/terminal`          | `mark_dim_hr` | ≈ 0.92×              |
| `migration/terminal_no_color` | `mark_dim_hr` | **≈ 1730×**          |
| `migration/browser`           | `mark_dim_hr` | ≈ 6.0×               |
| `migration/markdown`          | `mark_dim_hr` | (tree-only) 578 µs   |
| `migration/fold_only`         | `mark_dim_hr` | ≈ 18.2×              |

These ratios mix two unrelated costs: an eager-rasterization cliff
(graphics policy — owned by the sibling spec) and a structural span-aware
fold cost (real perf — owned here).

## Root-Cause Analysis

### Finding 1 — Terminal: HR rasterization is unconditional on the tree path *(graphics-policy owns the fix)*

This is the dominant terminal regression. The tree terminal renderer
rasterizes 20 PNGs (~275 µs each ≈ 5.5 ms) per `mark_dim_hr` render with
no policy by which a caller can opt out. The legacy path honors
`TerminalImageMode`; the tree path has no equivalent surface, and the
darkmatter entry point that maps `TerminalOptions →
TerminalRenderOptions` drops `opts.image_mode` entirely
(`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:189-202`).

**Fix is not in this spec.** See `../2026-05-26-graphics-policy/spec.md`
(items A-1 stopgap, B-1 framework fix). The perf cost numbers above are
the measurement that motivates the graphics-policy decision; the
remediation lives there because it is a cross-component policy choice,
not a local optimization. This spec's contribution to Finding 1 is the
measurement stick.

### Finding 2 — Browser: the 6× is fold cost, not SVG generation

The baseline note states the browser ratio is "dominated by 20 HR-rule SVG
generations on the tree side." **That is inaccurate.** The tree browser
renderer lowers a `ThematicBreak` to a plain `<hr>` void tag with
`data-hr-*` attributes (`renderable/src/tree/render/browser.rs:356-371`) —
it generates **no SVG**. The rich `<svg>` (CSS-variable driven,
`render_browser_svg`, `.../horizontal_rule/browser.rs:126-214`) is
produced only by the **legacy** HTML path.

So the browser 6× is dominated by the **span-aware fold** (`fold_only`
for `mark_dim_hr` is already 164 µs, ≈ 18× the plain fold) plus mark / dim
inline-node construction — structural costs shared with the markdown
target (578 µs). The HR contribution to the tree browser number is small;
the *fidelity gap* between plain `<hr>` and styled `<svg>` is owned by
the graphics-policy spec (B-3 / B-4).

The remaining browser perf signal is real and is addressed by Bucket 1
(QW-4 fold hygiene) below.

### Finding 3 — Fold: span-aware processor overhead

`fold_only` for `mark_dim_hr` is ≈ 18.2× the legacy parser drain (164 µs
vs 9 µs). This is the documented cost of
`fold_markdown_spanned_with_frontmatter` preserving byte ranges and
`darkmatter.hr` hints for 100 darkmatter-inline constructs. It is a real
structural cost every tree target inherits and is the lower bound for
browser / markdown renders of this fixture.

This is the **only finding whose fix is wholly inside this spec** — see
QW-4 below.

## Bucket 1 — Quick Wins (no risk, no framework or ergonomic change)

These are local, behavior-preserving, and confined to `biscuit-terminal`
(or to allocation hygiene already sanctioned by DMTR-7). None change a
public API, render output, or the tree contract. The first three help
*only when graphics-policy permits rasterization*; they are complementary
to that spec, not substitutes for it.

### QW-1: Content-addressed rasterization memo for repeated rules

`rasterize_svg_to_png` is pure: identical SVG bytes always produce
identical PNG bytes. A document with many rules sharing the same resolved
style / weight / width / color / cell-size produces identical SVGs. Add a
**bounded thread-local cache** keyed on the SVG bytes (or their hash)
inside `biscuit-terminal`, so a render pass that emits N identical rules
rasterizes once instead of N times.

- **Output:** byte-identical.
- **Scope:** internal to `biscuit-terminal`; no signature or tree change.
- **Cost envelope:** for `mark_dim_hr`, collapses up to 20 rasterizations
  toward the number of *distinct* rule appearances.
- **Only consideration:** a small, explicitly bounded memory footprint
  (cap the cache, e.g. LRU of N entries). It does not change behavior and
  adds no global configuration.
- **Interaction with graphics-policy:** complementary. If
  `GraphicsMode::Off` / `Structural` already suppresses rasterization,
  QW-1 has no work to memoize. If `Rich`, it shaves the cost of repeated
  identical rules. Either way the memo is safe.

### QW-2: SVG-string construction micro-optimizations

`render_image_svg` builds the rule SVG with several `format!` calls; the
`Waves` arm grows a `String` with `push_str(&format!(…))` in a loop
(`.../horizontal_rule/mod.rs:418-428`). Pre-size the buffer and write
segments with `write!` into a single `String`. Pure allocation hygiene,
behavior- and byte-preserving. Small benefit; safe.

### QW-3: Reuse `resvg::usvg::Options` instead of constructing per call

`rasterize_svg_to_png` builds `resvg::usvg::Options::default()` on every
call (`.../horizontal_rule/mod.rs:699`). HR SVGs contain no text, so the
default font database is constructed and never used. Investigate
constructing it once (e.g. a `thread_local!` / `OnceLock`-guarded
reusable `Options`) and reusing it across rasterizations.
Behavior-preserving if the options are identical to today's default.
(Verify `Options` reuse is sound for the pinned `resvg` version before
adopting.)

### QW-4: DMTR-7 allocation hygiene in the span-aware fold

The 18× `fold_only` ratio (Finding 3) is structural, but a
measurement-guided pass over `fold.rs` / `span.rs` for pre-sizing
`SpannedInlineEvent` buffers and avoiding redundant byte-range / hint-
string clones is squarely within the already-approved DMTR-7 posture.
These are local, behavior-preserving cleanups — not a rewrite. Treat as
opportunistic, profiler-led, and bounded.

**This is the only Bucket 1 item that helps the browser and markdown
targets** (Finding 2), because the fold is shared across all tree
targets. The terminal targets benefit too, layered with QW-1..QW-3.

## Bucket 2 — Framework: No-Color Fast Path

Only one framework-level perf item survives the carve-out. The graphics-
policy items (the old FC-1, FC-3) have moved to the sibling spec.

### FC-2: A no-color fast path for the tree terminal renderer

The parent spec already pre-commits to this ("`ColorDepth::None` … need
explicit fast paths" — `../2026-05-20-darkmatter-tree/spec.md`,
Performance Expectations). The regression is not HR-specific:
`small_prose` no-color is ≈ 155× (15 µs vs 99 ns) because the tree always
builds an owned `Document` and walks it, where legacy short-circuits. A
framework-level fast path would let the tree terminal renderer emit plain
text for `ColorDepth::None` (and no graphics) through a cheaper walk.

- **Benefit:** addresses the no-color regression across *all* fixtures,
  not just `mark_dim_hr`.
- **Downside:** a second terminal-lowering path to keep in sync with the
  full path; risk of behavioral divergence (wrapping, alignment, layout)
  between the two. Must still honor `Layout` / `LineWrapper` semantics or
  it will silently diverge from the full renderer. Meaningful maintenance
  cost; high value.
- **Composition with graphics-policy:** complementary. The graphics-
  policy fix (B-1 in the sibling spec) removes the *rasterization* cost
  on the no-color path; FC-2 removes the *walk / format* cost on the
  no-color path. Together, `mark_dim_hr` no-color drops from 5.50 ms to
  roughly the fold floor (~164 µs) plus cheap text assembly.

## Cross-Bucket Summary

| Item  | Target            | Removes          | Bucket | Note |
|-------|-------------------|------------------|--------|------|
| QW-1  | Terminal          | duplicate raster | 1 | Helps only when rules repeat *and* graphics fire |
| QW-2  | Terminal          | SVG-string allocs| 1 | Micro; only matters when graphics fire |
| QW-3  | Terminal          | per-call `Options` | 1 | Verify reuse sound; only matters when graphics fire |
| QW-4  | All targets       | fold allocs      | 1 | The only Bucket 1 item that helps browser / markdown |
| FC-2  | Terminal          | no-color walk    | 2 | All fixtures, not just HR; pairs with graphics-policy B-1 |

## Open Questions

- Is QW-1's repeated-rule memoization worth its bounded cache once
  graphics-policy ships a `GraphicsMode::Off` path? (Yes for `Rich`
  callers that render many rules; the cache should be opt-in if its
  memory footprint matters.)
- Before scheduling FC-2, what is the standard memory profiler for this
  workspace (still an open question from DMTR-6)?
- Does the QW-4 fold-hygiene pass need its own micro-benchmark group, or
  is `migration/fold_only` enough?

## Out of Scope

- Graphics / image policy on the render context. Owned by
  `../2026-05-26-graphics-policy/spec.md`.
- HR tier defaults, image-tier capability gating, browser HR SVG
  fidelity. All owned by the graphics-policy spec.
- Aggressive tuning the parent migration explicitly defers (arenas,
  interning, `Arc<str>`, lifetime-parametric nodes, broad `SmallVec`).
- Replacing `resvg` / the rasterization stack.
- Any change to `pulldown-cmark` parsing or the span-aware provenance
  contract beyond local allocation hygiene (QW-4).

## Related Specs

- `../2026-05-26-graphics-policy/spec.md` — owns Finding 1's remediation
  (graphics policy on the render context) and the tree browser HR
  fidelity decision (Finding 2's fidelity gap).
- `../2026-05-20-darkmatter-tree/spec.md` — the parent migration spec.
- `../2026-05-20-darkmatter-tree/baselines.md` — the recorded ratios.

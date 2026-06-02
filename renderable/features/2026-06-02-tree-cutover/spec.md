---
status: draft
---

# Tree Cutover: Retire the Bespoke Renderers

## Status

**Draft — direction approved, sequencing and acceptance gates to be settled.**
The decision to switch all rendering onto the render-tree pipeline and remove
the legacy bespoke renderers has been made. What remains is to (1) close the
remaining fidelity gaps so the tree path is a true superset of the bespoke
output, (2) establish a performance baseline once the tree path is
fidelity-complete, (3) flip every render entry point — darkmatter's Markdown
document pipeline *and* every renderable component in biscuit-terminal and
darkmatter — to the tree, (4) validate parity and performance, and only then
(5) delete the bespoke renderers.

Several sub-decisions are genuinely open and are recorded under
[Decisions To Be Made](#decisions-to-be-made) rather than guessed. Nothing in
the deletion phase happens until the gates in
[Acceptance Criteria](#acceptance-criteria) are met.

This spec is the umbrella for the cutover the prior specs were prerequisites
for. Its direct inputs:

- [`../2026-05-26-inline-span/spec.md`](../2026-05-26-inline-span/spec.md) —
  replaced the per-event span transport with the source-layer inline rewriter
  (done). Recovered `<mark>` element fidelity.
- [`../2026-05-26-block-extension/spec.md`](../2026-05-26-block-extension/spec.md) —
  lifted HR-attribute handling off the span transport (done). Its
  *Legacy `RuleProcessor` Retention* gate is the deletion gate this spec
  discharges.
- [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md) —
  owns the HR/image fidelity decisions that block this cutover. **Hard
  upstream dependency — see [Phase 0](#phase-0--fidelity-graphics-policy-first).**
- [`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md) —
  the perf hotspots surfaced by `migration_parity`.

## Background

### Two distinct cutovers

There are two independent migrations onto the tree renderer; this spec
completes **both** and is the only place they are sequenced together. See also
the "Two distinct tree cutovers" section in
[`../../docs/components.md`](../../docs/components.md).

1. **Component default render paths.** Whether an individual component's own
   `render()` routes through the tree. Most `biscuit-terminal` components have
   already cut over (`both avail, tree renders`); holdouts remain (see
   [Component Holdouts](#component-holdouts)).
2. **The darkmatter Markdown document pipeline.** `Markdown::as_html`,
   `Markdown::for_terminal`, and `DarkmatterPage::render` still run the legacy
   event-stream serializers (`output/html.rs`, `output/terminal.rs`,
   `RuleProcessor`). `DarkmatterPage::render` is pinned byte-for-byte to
   `for_terminal(default)`. The tree document entry points
   (`render_tree_html` / `render_tree_terminal` / `render_tree_markdown` in
   `darkmatter/lib/src/markdown/render_tree/entrypoints.rs`) are `pub(crate)`
   and reached only from tests. **This pipeline has not cut over.**

### Current performance state (2026-06-02, full `migration_parity` run)

Ratios are tree ÷ legacy; below 1.0 means the tree path is faster. Measured
with `--warm-up-time 1 --measurement-time 3 --sample-size 10`.

- **Terminal:** tree wins on every fixture (5–325× faster) **except**
  `mark_dim_hr` (≈ 1.14× slower — unconditional HR PNG rasterization).
- **Multi-target (`fold_once_multi_target`):** tree wins 1.6–154× — the
  architectural payoff (fold once, render N targets).
- **Browser:** tree is **2–11× slower** across the board (worst:
  `large_table` ≈ 11×; parity only on `large_code_block`). This is the main
  perf blocker.
- **Inline fold path:** the inline-span cutover was perf-neutral
  (`mark_dim_hr` fold ≈ 170 µs, ≈ unchanged from the old span processor), and
  the production fold now pays a small linear source-scan tax on every
  document (+0.6 to +48 µs depending on size). Acceptable; recorded in
  `../_completed/2026-05-20-darkmatter-tree/baselines.md`.

### Fidelity gaps blocking "no regressions"

| Gap | Target | Owner | Notes |
|---|---|---|---|
| HR rasterization not opt-out-able (`TerminalImageMode` dropped by tree entry point) | Terminal | graphics-policy ✅ | Resolved: raster gated to `Rich`; `TerminalImageMode::Never → Off`. |
| HR lowers to plain `<hr data-hr-*>` vs legacy CSS-variable `<svg>` | Browser | graphics-policy ✅ | Resolved: styled `<svg>` restored at `Vector`+ (B-3). |
| Mermaid deferred on the tree path | Terminal/Browser | graphics-policy ✅ | Resolved: promoted `Code` node — static `<svg>` browser, raster terminal. |
| `<mark>` element recovery (`<span class="mark">` → `<mark>`) | Browser | inline-span (done) | **Deliberate improvement, not a regression** — snapshot updates allowed. |
| `large_table` browser path ≈ 11× slower | Browser | this spec (perf) | Not graphics; a tree browser renderer hotspot. |
| `StyleWarning` not surfaced through tree entry points | all | block-extension (deferred) | Legacy strict-style warnings; parity question. |

## Goals

- Make the tree renderer a **fidelity superset** of every bespoke renderer it
  replaces (parity, or a deliberate documented improvement).
- Establish a recorded performance baseline at the moment the tree path is
  fidelity-complete, before any entry point flips.
- Flip **all** rendering — darkmatter Markdown document pipeline and every
  renderable component in biscuit-terminal and darkmatter — to the tree.
- Validate parity and performance against the recorded gates.
- Remove the legacy/bespoke renderers entirely once validated.

## Non-Goals

- Designing new graphics policy — owned by the graphics-policy spec. This spec
  consumes its decisions; it does not make them.
- New rendering features beyond reaching bespoke parity.
- Replacing pulldown-cmark or changing the tree IR beyond what parity requires.
- Changes to compose, frontmatter, schema, or `style:` wiring except where a
  flip exposes a parity gap.

## Acceptance Criteria

These gate the **deletion** phase. All must hold (this is the policy recorded
in [`../../docs/components.md`](../../docs/components.md) "Removing the bespoke
renderers"):

1. **Darkmatter pipeline on the tree.** `Markdown::as_html`,
   `Markdown::for_terminal`, and `DarkmatterPage::render` route through the
   render-tree document renderers.
2. **Every renderable component on the tree.** Each component is
   `tree render only`, *or* is explicitly exempted with documented
   justification (see [the Prose / non-structural decision](#decisions-to-be-made)).
3. **No functional or fidelity regressions** versus bespoke on any target.
   Output parity, or a deliberate documented improvement (e.g. `<mark>`).
4. **Net performance trend toward faster.** The corpus-wide trend must
   improve; mild localized regressions are acceptable so long as the general
   direction is faster. The concrete metric is a
   [Decision To Be Made](#decisions-to-be-made).

## Phases

Ordered so each phase lands on a green tree and nothing flips before its
fidelity gap is closed.

### Phase 0 — Fidelity (graphics-policy first)

**Hard prerequisite.** Implement graphics-policy (architecture approved), then
close the remaining non-graphics fidelity gaps.

0a. **Implement graphics-policy** — see
   [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md)
   (architecture approved). `GraphicsMode { Off, Vector, Rich }` on the
   per-target render contexts; terminal raster gated to `Rich` with
   `TerminalImageMode::Never → Off`; styled HR `<svg>` restored at `Vector`+;
   Mermaid brought on-tree as a promoted `Code` node (static `<svg>` browser,
   raster terminal). This clears the terminal `mark_dim_hr` regression, the
   browser HR fidelity gap, and the Mermaid deferral in one piece of work.
0b. **Close remaining non-graphics fidelity gaps:** `StyleWarning` surfacing
   through tree entry points (or a documented deferral), and any parity gap the
   `render_tree_parity` / `render_tree_hr_snapshots` tests surface.
0c. **Prose prerequisites** (from
   [`../2026-06-02-prose-tree/spec.md`](../2026-06-02-prose-tree/spec.md), shared
   so other components/darkmatter benefit): add `inverse` to
   `renderable::style::TextEmphasis` with terminal SGR 7 / browser / markdown
   lowering, and extend the shared markdown renderer to lower an inline `Style`
   to MarkdownPlus inline-HTML `<span style="…">`.

Exit gate: the tree path produces parity-or-better output on every target for
the full fixture corpus.

### Phase 1 — Baseline

Once Phase 0 is fidelity-complete, capture the authoritative baseline **before**
any entry point flips:

```bash
cargo bench -p darkmatter --bench migration_parity -- --save-baseline pre-cutover-2026-06-02
cargo bench -p biscuit-terminal --bench render_tree -- --save-baseline pre-cutover-2026-06-02
```

Record middle estimates in `baselines.md` and reference them in the cutover
PRs. This is the line every later run is compared against for the Acceptance
Criteria #4 trend gate.

> **Note — component comparison bench drift.** The
> `component_render_path_comparison` group in
> `biscuit-terminal/lib/benches/render_tree.rs` no longer compares bespoke vs
> tree: all six components' `render()` already routes through the tree, so both
> arms measure the tree path. It is still a valid tree-perf signal. A true
> historical bespoke-vs-tree comparison requires a pre-flip commit. This is
> noted so the baseline is read correctly, not as a blocker.

### Phase 2 — Flip the darkmatter document pipeline

- Promote the `render_tree_*` entry points from `pub(crate)` to `pub`.
- Route `Markdown::as_html`, `Markdown::for_terminal`, and
  `DarkmatterPage::render` through the tree document renderers.
- Update `DarkmatterPage::render`'s byte-for-byte `for_terminal` parity tests
  to the tree output (parity-or-better; the `<mark>` change is expected).
- Behind a feature flag for a deprecation window, or hard flip — a
  [Decision To Be Made](#decisions-to-be-made).

### Phase 3 — Flip remaining component holdouts

Move each holdout's default `render()` to the tree, or exempt it with
justification. See [Component Holdouts](#component-holdouts).

### Phase 4 — Validate

- Full test corpus green, including `render_tree_parity.rs` and
  `render_tree_hr_snapshots.rs`.
- Re-run both benches against the Phase-1 baseline; confirm Acceptance
  Criteria #4 (net trend toward faster, mild localized regressions allowed).
- Confirm no caller constructs a legacy renderer or `RuleProcessor`.

### Phase 5 — Delete the bespoke renderers

Once Phases 2–4 are validated and Acceptance Criteria hold:

- Delete `output/html.rs` and `output/terminal.rs` legacy serializers and
  `RuleProcessor` (discharging the block-extension spec's removal gate).
- Remove component bespoke render bodies / `render_bespoke` / `fallback_render`
  compatibility hooks for components that flipped in Phase 3.
- Remove now-dead support types; ensure `parse_hr_attribute_block` and
  `scan_inline_hr_warnings` remain the single source of truth for HR parsing.

## Component Holdouts

Default render path still bespoke or off-tree (from
[`../../docs/components.md`](../../docs/components.md)):

| Component | Crate | State | Cutover action |
|---|---|---|---|
| `FileSystem` (terminal) | biscuit-terminal | tree exists, terminal renders bespoke | Connector-list `Style` lowering + icon-spacing parity, then flip. |
| `YamlBlock` | darkmatter | both avail, old renders | Flip default to tree projection. |
| `Prose` | biscuit-terminal | component-local `ProseDocument` IR | **Resolved:** full collapse to the shared tree — see [`../2026-06-02-prose-tree/spec.md`](../2026-06-02-prose-tree/spec.md). |
| `GraphExpression`, `MermaidDiagram`, `TerminalImage`, `Status`, `InlineContent`, `PadLeft`, `PadRight`, `HorizontalRule`, `DarkmatterPage`, `FileTree` | biscuit-terminal / darkmatter | `no changes` (no tree projection) | **Decision:** which need a tree projection vs are exempt (see below). |

## Decisions To Be Made

These need brainstorming/sign-off; they do not change the overall direction.

1. **Browser HR fidelity.** ✅ **Resolved — restore styled `<svg>` at
   `Vector`+** (graphics-policy B-3; B-4 rejected). See
   [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md).
2. **Terminal graphics opt-out shape.** ✅ **Resolved — framework
   `GraphicsMode` on the render context** (graphics-policy B-1; Bucket A
   stopgaps dropped). Raster gated to `Rich`; `TerminalImageMode::Never → Off`.
3. **Concrete perf gate for Acceptance Criteria #4.** Define the metric, e.g.
   "corpus geomean of tree/legacy ≤ 1.0 and no single fixture regresses beyond
   N×." Pick the threshold (browser `large_table` is currently 11× — is it
   fixed pre-cutover, or accepted under the tolerance?).
4. **`Prose` exemption.** ✅ **Resolved — full collapse onto the shared tree.**
   `Prose`'s parser will emit `RenderNode` directly and `ProseDocument` is
   deleted; the two shared-tree prerequisites (`inverse` on `TextEmphasis`,
   MarkdownPlus inline-`Style` lowering) move to Phase 0c. See
   [`../2026-06-02-prose-tree/spec.md`](../2026-06-02-prose-tree/spec.md).
5. **Non-structural components.** `PadLeft`/`PadRight`, `InlineContent`,
   `TerminalImage`, `Status`, `DarkmatterPage`, `FileTree` may have no
   document-structure tree equivalent. Define the exemption criteria so
   "every component on the tree" is an achievable bar, not an impossible one.
6. **Mermaid on the tree.** ✅ **Resolved — designed in graphics-policy** as a
   promoted `Code` node (static `<svg>` browser, raster terminal; interactive
   mermaid.js an orthogonal opt-in). Built in Phase 0a, before cutover.
7. **`StyleWarning` surfacing.** Thread strict-style warnings through the tree
   entry points (parity with legacy), or keep the existing
   `scan_inline_hr_warnings` preflight surface and document the difference.
8. **Deprecation window vs hard flip.** Keep legacy behind a feature flag for a
   release before deletion, or flip-and-delete once validated.
9. **`large_table` browser hotspot.** Investigate and fix the 11× regression
   (preferred) or accept it under the Decision-3 tolerance.

## Out of Scope

- Graphics policy design (graphics-policy spec).
- New IR variants beyond parity needs.
- Parser replacement.
- Public API shape changes beyond promoting the tree entry points.

## Related Specs

- [`../2026-06-02-prose-tree/spec.md`](../2026-06-02-prose-tree/spec.md) —
  resolves Decision #4; supplies Phase 0c prerequisites and a Phase 3 holdout.
- [`../2026-05-26-inline-span/spec.md`](../2026-05-26-inline-span/spec.md)
- [`../2026-05-26-block-extension/spec.md`](../2026-05-26-block-extension/spec.md)
- [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md)
- [`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md)
- [`../_completed/2026-05-20-darkmatter-tree/spec.md`](../_completed/2026-05-20-darkmatter-tree/spec.md)
- [`../_completed/2026-05-20-darkmatter-tree/baselines.md`](../_completed/2026-05-20-darkmatter-tree/baselines.md)

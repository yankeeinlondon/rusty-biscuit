---
clarified: false
reviewed: false
review_iterations: 0
status: draft — unscheduled
---

# Per-Pane Borders for SplitPane — Draft Design Specification

> **Status:** Draft, unscheduled. Nothing here is implemented or planned yet —
> the code blocks are API sketches, not final signatures. The shape question
> **Q1 is resolved**: ship the Option 2 render wrapper
> ([§4.2](#42-option-2--render-wrapper-with-optional-per-pane-chrome-recommended)).
> The remaining open questions in [§7](#7-open-questions) (Q2–Q6) still need a
> clarifying pass before this moves to planning.
> **Date:** 2026-06-30
> **Area:** `biscuit-tui/lib`
> **Precedes / depends on:** the geometry-only `SplitPane`
> ([`2026-06-24-split-pane/spec.md`](../2026-06-24-split-pane/spec.md)) and
> `FrameChrome` ([`docs/components/frame_chrome.md`](../../docs/components/frame_chrome.md)).

## 1. Summary

Today `SplitPane` is a geometry-only primitive: it computes two child rects and
draws nothing ([split-pane spec §3](../2026-06-24-split-pane/spec.md)). A single
border *around both panes* is already achievable by composing an outer
`FrameChrome` with `SplitPane::split` on its inner rect (the "Option A"
composite-widget pattern documented in the `biscuit-tui` skill). What is **not**
yet ergonomic is a border around **one or both individual panes**.

This feature adds an opt-in way to draw a `FrameChrome`-style border around each
pane of a split, reusing the existing `BorderStyle` / `FrameChromeConfig`
vocabulary so there is one border language across the crate.

This is the natural realization of the "render wrapper" that the split-pane spec
deliberately deferred ([split-pane spec §8.1](../2026-06-24-split-pane/spec.md)):
once you render *into* the panes, per-pane chrome is the first thing that pays
for the wrapper.

## 2. Motivation

- **Master/detail and sidebar layouts read better with framed panes.** A
  bordered, titled detail pane next to a bordered master list is a common,
  legible TUI shape. Doing it today means the caller hand-wraps each child in
  its own `FrameChrome` after `split()` — workable but boilerplate-heavy and
  easy to get subtly wrong (which rect feeds which `FrameChrome`, whose padding
  applies where).
- **Titles per pane.** `FrameChrome` already supports top/bottom border labels;
  surfacing that per pane gives "▏Files▕ | ▏Preview▕" for free.
- **One border vocabulary.** Callers should not learn a second border API for
  panes. Per-pane borders must be expressed with the same `BorderStyle`,
  `border_label`, `border_style`, and `Padding` types `FrameChrome` already
  uses.

Non-goals (for this cut):

- **No seamed/merged border junctions.** Adjacent pane borders render as two
  parallel lines, not merged `├`/`┤`/`┼` glyphs (see
  [§5.1](#51-no-merged-junctions-the-double-line-seam)). A glyph-join pass is a
  separate, larger feature.
- **No draggable divider, no divider glyph.** Those remain future split-pane
  enhancements ([split-pane spec §8](../2026-06-24-split-pane/spec.md)),
  independent of per-pane chrome.
- **No new `question` CLI command.** `SplitPane` is a library layout primitive
  with no CLI surface; per-pane borders inherit that. (A future
  `--pane-border`-style flag on a hypothetical split command is explicitly out
  of scope.)
- **No focus/event handling.** Like `SplitPane`, this stays a layout/render
  concern; the embedding app owns focus and the event loop.

## 3. Relationship to existing pieces

| Piece | Role today | This feature |
| :--- | :--- | :--- |
| `SplitPane::split(area) -> (Rect, Rect)` | Pure geometry. Always available. | **Unchanged.** Per-pane borders build *on top of* `split()`. |
| `FrameChrome` / `FrameChromeConfig` | Wraps one `StatefulWidget` with border/margin/padding/title. | **Reused per pane.** Each pane is conceptually its own `FrameChrome`. |
| Outer `FrameChrome` ("Option A") | One border around the whole split. | **Composes.** Outer border + per-pane borders can stack (with the double-line seam caveat). |

The decisive question this spec must settle is **whether per-pane borders ship
as a documented composition pattern (just docs + an example), a thin render
wrapper, or a fuller stateful widget** — see [§4](#4-design-options) and
[§7](#7-open-questions).

## 4. Design options

### 4.1 Option 1 — docs-only pattern (lowest cost)

Ship no new type. Document and example-test the "wrap each pane's child in its
own `FrameChrome` after `split()`" recipe:

```rust
let (left, right) = SplitPane::new()
    .with_direction(SplitDirection::Horizontal)
    .split(area);

let left_cfg  = FrameChromeConfig { border_label: Some("Files".into()),   ..Default::default() };
let right_cfg = FrameChromeConfig { border_label: Some("Preview".into()), ..Default::default() };

FrameChrome::from_config(LeftWidget,  &left_cfg ).render(left,  buf, &mut state.0);
FrameChrome::from_config(RightWidget, &right_cfg).render(right, buf, &mut state.1);
```

- **Pros:** zero new API; strictly additive; ships immediately; no
  heterogeneous-state plumbing.
- **Cons:** boilerplate at every call site; no single place to express "border
  the left pane only"; nothing stops mismatched configs.

### 4.2 Option 2 — render wrapper with optional per-pane chrome (recommended)

Introduce the deferred render wrapper, but make per-pane chrome its headline
feature. A **named** state struct (per the split-pane spec's resolved decision —
never a positional tuple):

```rust
/// Renders two child widgets into a split, each optionally framed by its own
/// chrome. The wrapper owns no border itself beyond what each pane requests;
/// stack an outer `FrameChrome` for a perimeter border.
pub struct SplitPaneFramed<A, B> {
    pub split: SplitPane,
    pub first: A,
    pub second: B,
    /// Per-pane chrome. `None` ⇒ that pane renders flush (no border).
    pub first_chrome: Option<FrameChromeConfig>,
    pub second_chrome: Option<FrameChromeConfig>,
}

pub struct SplitPaneState<SA, SB> {
    pub first: SA,
    pub second: SB,
}

impl<A, B> StatefulWidget for SplitPaneFramed<A, B>
where
    A: StatefulWidget,
    B: StatefulWidget,
{
    type State = SplitPaneState<A::State, B::State>;
    // split() the area, then for each pane: if Some(cfg) wrap the child in
    // FrameChrome::from_config, else render the child directly into the rect.
}
```

- **Pros:** one call site; "border one pane only" is a single `Option`; reuses
  `FrameChromeConfig` wholesale; realizes the deferred wrapper with a concrete
  motivating use case.
- **Cons:** introduces the heterogeneous-state plumbing the split-pane spec
  flagged as the wrapper's main cost
  ([split-pane spec §5.3](../2026-06-24-split-pane/spec.md)); the
  master/detail pattern (derived detail pane, no independent state) does **not**
  fit a two-independent-children wrapper and must keep using bare `split()`.

### 4.3 Option 3 — full stateful split widget

A larger widget that also owns focus, divider glyph, and ratio-in-state for
future dragging. **Out of scope here** — explicitly deferred to the broader
split-pane future-enhancements list. Per-pane borders should not wait on it.

**[DECISION — Q1 — RESOLVED] — ship Option 2 (the render wrapper).** Per-pane
borders ship as the `SplitPaneFramed` render wrapper in
[§4.2](#42-option-2--render-wrapper-with-optional-per-pane-chrome-recommended),
with a **named** `SplitPaneState` (never a positional tuple), reusing
`FrameChromeConfig` per pane. Option 1's bare-`split()` + manual `FrameChrome`
recipe is **retained in the docs** as the recommended path for the
master/detail case, where the derived-detail pane does not fit a
two-independent-children wrapper ([§5.3 of the split-pane
spec](../2026-06-24-split-pane/spec.md), and Q5 below). The two coexist: the
wrapper for two independent framed children, bare `split()` for derived panes.

## 5. Behavior & semantics (for the recommended wrapper)

### 5.1 No merged junctions (the double-line seam)

ratatui does not merge adjacent `Block` borders into shared junction glyphs.
With `gap = 0`, a bordered left pane and a bordered right pane render **two
adjacent vertical lines** at the seam, not a single shared `│`. Options to
make this acceptable:

- Document it and recommend `gap = 1` so the two borders are visibly separated
  rather than awkwardly doubled.
- A future "seamed" mode that post-processes the buffer to replace adjacent
  border cells with junction glyphs (`├`/`┤`/`┬`/`┴`/`┼`). This is a real
  glyph-join pass over the shared `Buffer` and is **out of scope** for this
  cut — but the API should not foreclose it (e.g. leave room for a
  `with_seamed(true)` builder later).

This caveat must be settled before the API locks in, because "should adjacent
borders merge?" changes the data the wrapper needs.

### 5.2 Inner-rect math

Each pane's `FrameChrome` claims its own one-cell border perimeter and padding
**inside** its pane rect, exactly as `FrameChrome` does standalone (margin →
border → padding → inner). The child widget therefore renders into
`pane_rect` shrunk by that pane's chrome — independent of the other pane.

### 5.3 Degenerate areas

A pane too small to fit its border (< 2 cells on the bordered axis) must not
panic or overflow; it should degrade the same way `FrameChrome` already does on
a tiny rect (border clipped to the available cells). This inherits
`FrameChrome`'s existing small-area behavior rather than inventing new rules —
to be verified against `FrameChrome`'s current clipping at implementation time.

### 5.4 Composition with an outer border

An outer `FrameChrome` (the "one border around both" pattern) and per-pane
borders stack: outer perimeter, then each inner pane framed. With `gap = 0` the
inner pane borders sit one cell inside the outer border, producing parallel
lines (the same seam caveat as [§5.1](#51-no-merged-junctions-the-double-line-seam)).

## 6. Definition of done & testing plan (proposed)

- **Geometry/render unit tests (L1, `TestBackend`):**
  - A pane with `Some(chrome)` draws its border glyphs at the pane rect's
    perimeter; its child renders in the shrunk inner rect.
  - A pane with `None` chrome renders the child flush into the full pane rect.
  - "Border one pane only" — `first_chrome: Some`, `second_chrome: None` —
    produces a border on the first pane and none on the second.
  - Per-pane `border_label` / `bottom_label` render in the correct pane.
  - The double-line seam at `gap = 0` is asserted (two adjacent border columns),
    pinning the documented non-merge behavior so a future seamed mode is a
    conscious change.
  - Tiny-area panes never panic and never overflow (inherits `FrameChrome`).
- **Cross-platform:** pure layout + buffer writes; identical on macOS, Linux,
  Windows. No platform-specific paths. No real-terminal harness needed.
- **Docs:**
  - Extend `docs/components/split_pane.md` (or the layout/core docs page) with a
    per-pane-border section and the master/detail caveat (use bare `split()`,
    not the wrapper, when the detail pane is derived).
  - Update the `biscuit-tui` skill `SplitPane` section to point at the shipped
    API instead of this draft.
  - `CHANGELOG.md` under `Unreleased` (new public library API if Option 2).
- **Verification:** `just test`; `just lint` if public docs/exports change. Do
  **not** run `cargo fmt` write-mode unless explicitly requested.

## 7. Open questions

| ID | Question | Notes |
| :--- | :--- | :--- |
| ~~**Q1**~~ | ~~Ship as docs-only recipe, render wrapper, or both?~~ | **RESOLVED** ([§4.3](#43-option-3--full-stateful-split-widget) decision marker): ship the Option 2 render wrapper (`SplitPaneFramed` + named `SplitPaneState`); retain the bare-`split()` docs recipe for master/detail. |
| **Q2** | Does the wrapper revive the deferred `SplitPaneWidget` decision wholesale, or is per-pane chrome a distinct type? | Reconcile with [split-pane spec §8.1](../2026-06-24-split-pane/spec.md) (named `SplitPaneState`, strictly additive). |
| **Q3** | Default seam behavior at `gap = 0`: doubled lines (document) or auto-bump to `gap = 1` when both panes are bordered? | Affects whether the wrapper silently mutates the caller's gap. |
| **Q4** | Should merged junction glyphs (`├`/`┼`) be in-scope now or explicitly future? | Recommendation: future. Keep the API open (`with_seamed`) without building it. |
| **Q5** | Per-pane border on the **master/detail** pattern — does the derived-detail pane keep using bare `split()` + manual `FrameChrome`, given the wrapper's two-independent-children shape? | Likely yes; document the boundary so callers pick the right tool. |
| **Q6** | Outer + per-pane border composition — is the parallel-line result ([§5.4](#54-composition-with-an-outer-border)) acceptable, or does it argue for the seamed pass sooner? | Visual judgement call; gather a real layout before deciding. |

---

*Draft, unscheduled. No clarification or review pass has run yet. The code
blocks are API sketches, not final signatures.*

---
clarified: false
reviewed: true
review_iterations: 0
status: ready for planning and implementation
---

# Per-Pane Borders for SplitPane — Design Specification

> **Status:** Ready for planning and implementation. Nothing here is implemented
> yet; the code blocks are API sketches, not final signatures. Review resolved
> the remaining shape questions: per-pane borders ship as the deferred
> `SplitPaneWidget` render wrapper, the default seam behavior is documented
> doubled borders, and merged junction glyphs remain future work.
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

This is the natural realization of the `SplitPaneWidget<A, B>` render wrapper
that the split-pane spec deliberately deferred
([split-pane spec §8.1](../2026-06-24-split-pane/spec.md)): once you render
*into* the panes, per-pane chrome is the first thing that pays for the wrapper.

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
- **No change to `FrameChrome` behavior.** Pane chrome must reuse
  `FrameChrome` exactly: margin outside that pane's border, optional border,
  padding inside the border, `bottom_label` only when the resolved border has a
  bottom segment, and clipped rendering on tiny rects.

## 3. Relationship to existing pieces

| Piece | Role today | This feature |
| :--- | :--- | :--- |
| `SplitPane::split(area) -> (Rect, Rect)` | Pure geometry. Always available. | **Unchanged.** Per-pane borders build *on top of* `split()`. |
| `FrameChrome` / `FrameChromeConfig` | Wraps one `StatefulWidget` with border/margin/padding/title. | **Reused per pane.** Each pane is conceptually its own `FrameChrome`. |
| Outer `FrameChrome` ("Option A") | One border around the whole split. | **Composes.** Outer border + per-pane borders can stack; this intentionally produces nested/parallel lines rather than merged junctions. |

**[DECISION — D1 — RESOLVED]** Per-pane borders ship as a render wrapper named
`SplitPaneWidget<A, B>`, matching the deferred public shape in the split-pane
spec. The docs-only composition pattern remains documented for derived
master/detail panes, but it is not the only shipped surface.

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

### 4.2 Option 2 — render wrapper with optional per-pane chrome (selected)

Introduce the deferred render wrapper, but make per-pane chrome its headline
feature. A **named** state struct (per the split-pane spec's resolved decision —
never a positional tuple):

```rust
/// Renders two child widgets into a split, each optionally framed by its own
/// chrome. The wrapper owns no border itself beyond what each pane requests;
/// stack an outer `FrameChrome` for a perimeter border.
pub struct SplitPaneWidget<A, B> {
    pub split: SplitPane,
    pub first: A,
    pub second: B,
    /// Per-pane chrome. `None` means no chrome at all: no margin, border, or
    /// padding. `Some(FrameChromeConfig::default())` still applies the default
    /// one-cell padding because that is `FrameChromeConfig`'s existing
    /// contract.
    pub first_chrome: Option<FrameChromeConfig>,
    pub second_chrome: Option<FrameChromeConfig>,
}

pub struct SplitPaneState<SA, SB> {
    pub first: SA,
    pub second: SB,
}

impl<A, B> StatefulWidget for SplitPaneWidget<A, B>
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

**[DECISION — D1 — RESOLVED] — ship Option 2 (the render wrapper).** Per-pane
borders ship as the `SplitPaneWidget` render wrapper in
[§4.2](#42-option-2--render-wrapper-with-optional-per-pane-chrome-selected),
with a **named** `SplitPaneState` (never a positional tuple), reusing
`FrameChromeConfig` per pane. The selected name intentionally matches
`SplitPaneWidget<A, B>` from the earlier split-pane spec instead of introducing
a second wrapper type such as `SplitPaneFramed`; per-pane chrome is the
motivating feature for the deferred wrapper, not a separate container family.
Option 1's bare-`split()` + manual `FrameChrome` recipe is **retained in the
docs** as the recommended path for the master/detail case, where the
derived-detail pane does not fit a two-independent-children wrapper
([§5.3 of the split-pane spec](../2026-06-24-split-pane/spec.md)). The two
coexist: the wrapper for two independent framed children, bare `split()` for
derived panes.

## 5. Behavior & semantics (for the recommended wrapper)

### 5.1 No merged junctions (the double-line seam)

ratatui does not merge adjacent `Block` borders into shared junction glyphs.
With `gap = 0`, a bordered left pane and a bordered right pane render **two
adjacent vertical lines** at the seam, not a single shared `│`.

**[DECISION — D2 — RESOLVED]** The wrapper must not silently mutate
`SplitPane::gap`. `gap = 0` means the panes are flush, so bordered panes produce
the doubled seam. Docs should recommend `gap = 1` when callers want a visual
gutter between independent pane borders.

**[DECISION — D3 — RESOLVED]** A future "seamed" mode may post-process the
buffer to replace adjacent border cells with junction glyphs
(`├`/`┤`/`┬`/`┴`/`┼`), but that glyph-join pass is out of scope. The API should
not foreclose it; reserving room for a future `with_seamed(true)` builder is
enough for this cut.

### 5.2 Inner-rect math

Each pane's `FrameChrome` claims its own margin, one-cell border perimeter, and
padding **inside** the rect produced by `SplitPane::split`, exactly as
`FrameChrome` does standalone (margin → border → padding → inner). The child
widget therefore renders into `pane_rect` shrunk by that pane's chrome —
independent of the other pane.

`None` chrome and `Some(FrameChromeConfig::default())` are intentionally
different:

- `None` renders the child flush into the full pane rect.
- `Some(FrameChromeConfig::default())` draws no border and no margin, but still
  applies the existing default `Padding::uniform(1)`.

Implementations must preserve that distinction and must not use
`FrameChromeConfig::is_empty()` to collapse `Some(default)` into `None`.

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

**[DECISION — D4 — RESOLVED]** This composition is acceptable for the first cut.
Callers that dislike the dense lines can add outer padding, set a pane `gap`, or
choose only one chrome layer. The wrapper must not special-case an ancestor
`FrameChrome`; it has no reliable way to know whether it is nested inside one.

### 5.5 Master/detail boundary

`SplitPaneWidget<A, B>` is for two independent child widgets with two
independent state values. It is not the right abstraction for the existing
master/detail pattern where the detail pane is derived from
`ChooseOneState::active_option()` each frame and may have no state of its own.

**[DECISION — D5 — RESOLVED]** Document both paths:

- Use `SplitPaneWidget` when both panes are real widgets with independent state.
- Use bare `SplitPane::split()` plus manual `FrameChrome::from_config(...)`
  when one pane is derived from the other, when render order matters, or when
  the caller needs to pass borrowed data into the second pane at draw time.

This is not a limitation of per-pane borders; it is the same
heterogeneous/derived-state boundary that caused the original split-pane spec to
ship geometry first.

## 6. Public API and exports

This feature adds public library API only:

- `lib/src/core/split_pane_widget.rs` (or the existing `split_pane.rs` if the
  implementation stays small) defines `SplitPaneWidget<A, B>` and
  `SplitPaneState<SA, SB>`.
- `lib/src/core/mod.rs` re-exports `SplitPaneWidget` and `SplitPaneState`.
- `lib/src/prelude.rs` re-exports `SplitPaneWidget` and `SplitPaneState`.
- `lib/tests/public_api_names.rs` proves both names are available from the
  expected public surfaces.

No new `question` command, flags, or shell completions are added. If CLI docs
are touched, they should only clarify that SplitPane and SplitPaneWidget are
library layout primitives.

## 7. Definition of done & testing plan

- **Geometry/render unit tests (L1, `TestBackend`):**
  - A pane with `Some(chrome)` draws its border glyphs at the pane rect's
    perimeter; its child renders in the shrunk inner rect.
  - A pane with `None` chrome renders the child flush into the full pane rect.
  - `Some(FrameChromeConfig::default())` applies padding while `None` does not.
  - "Border one pane only" — `first_chrome: Some`, `second_chrome: None` —
    produces a border on the first pane and none on the second.
  - Per-pane `border_label` / `bottom_label` render in the correct pane, and
    `bottom_label` remains ignored when the pane's resolved border has no
    bottom segment.
  - The double-line seam at `gap = 0` is asserted (two adjacent border columns),
    pinning the documented non-merge behavior so a future seamed mode is a
    conscious change.
  - `gap = 1` leaves a blank gutter between framed panes; the wrapper does not
    auto-bump a zero gap.
  - Outer `FrameChrome` plus per-pane chrome renders nested/parallel borders
    without panicking or overwriting outer corners.
  - Tiny-area panes never panic and never overflow (inherits `FrameChrome`).
  - `SplitPaneState<SA, SB>` forwards state by name, with mutations isolated to
    the correct child.
- **Cross-platform:** pure layout + buffer writes; identical on macOS, Linux,
  Windows. No platform-specific paths. No real-terminal harness needed.
- **Docs:**
  - Extend `docs/components/split_pane.md` (or the layout/core docs page) with a
    per-pane-border section, the doubled-seam behavior, the `None` versus
    `Some(default)` chrome distinction, and the master/detail caveat (use bare
    `split()`, not the wrapper, when the detail pane is derived).
  - Update the `biscuit-tui` skill `SplitPane` section to point at the shipped
    API instead of this specification.
  - `CHANGELOG.md` under `Unreleased` for the new public library API.
- **Verification:** `just test`; `just lint` if public docs/exports change. Do
  **not** run `cargo fmt` write-mode unless explicitly requested.

## 8. Resolved questions

| ID | Question | Notes |
| :--- | :--- | :--- |
| ~~**Q1**~~ | ~~Ship as docs-only recipe, render wrapper, or both?~~ | **RESOLVED:** ship the Option 2 render wrapper plus docs for the manual composition path. |
| ~~**Q2**~~ | ~~Does the wrapper revive the deferred `SplitPaneWidget` decision wholesale, or is per-pane chrome a distinct type?~~ | **RESOLVED:** use `SplitPaneWidget<A, B>` and named `SplitPaneState<SA, SB>`, matching the prior split-pane spec. Do not introduce `SplitPaneFramed`. |
| ~~**Q3**~~ | ~~Default seam behavior at `gap = 0`: doubled lines or auto-bump to `gap = 1` when both panes are bordered?~~ | **RESOLVED:** doubled lines. Do not mutate `gap`; recommend `gap = 1` in docs when a gutter is desired. |
| ~~**Q4**~~ | ~~Should merged junction glyphs (`├`/`┼`) be in-scope now or explicitly future?~~ | **RESOLVED:** future. Leave API room for `with_seamed(true)` but do not build a glyph-join pass now. |
| ~~**Q5**~~ | ~~Per-pane border on the master/detail pattern — does the derived-detail pane keep using bare `split()` + manual `FrameChrome`?~~ | **RESOLVED:** yes. The wrapper is for independent children; derived panes keep using bare geometry plus manual chrome. |
| ~~**Q6**~~ | ~~Outer + per-pane border composition — is the parallel-line result acceptable, or does it argue for the seamed pass sooner?~~ | **RESOLVED:** acceptable for this cut. The wrapper must not special-case ancestor chrome. |

---

*Reviewed inline on 2026-06-30. The code blocks are API sketches, not final
signatures.*

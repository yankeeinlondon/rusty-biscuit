---
clarified: claude/claude-opus-4-8
reviewed: true
review_iterations: 1
status: ready for planning and implementation
---

# SplitPane — Design Specification

> **Status:** Ready for planning and implementation — open architectural
> questions have now been **resolved** through review (see
> [§9](#9-open-questions--decisions)). Decision markers below read
> **[DECISION — Dn — RESOLVED]**.
> **Date:** 2026-06-24
> **Area:** `biscuit-tui/lib`

> **Inline review note:** This review keeps the geometry-only v1 decision,
> tightens public API placement/export expectations, removes an accidental
> agent-skill instruction from the testing section, and makes the companion
> `ChooseOneState` accessor work explicit enough to plan without changing
> `ChoiceOption`'s data model.

## 1. Summary

`SplitPane` is a **layout** widget that divides an available `Rect` into two
panes along a single axis. It is the first *layout* primitive in `biscuit-tui`
— unlike the existing components (`TextInput`, `ChooseOne`, …) it captures no
value and handles no input. Its job is purely spatial: take one rectangle, an
orientation, and a split ratio, and produce two child rectangles.

Two knobs, both required by the brief:

1. **Orientation** — `Vertical` stacks one pane on top of the other;
   `Horizontal` places them side-by-side.
2. **Split ratio** — how much relative space each pane receives; **defaults to
   a 50/50 split**.

It sits in the same conceptual family as `FrameChrome`: a *container* that
arranges other widgets rather than an input component. See
[`docs/components/frame_chrome.md`](../../docs/components/frame_chrome.md) for
the precedent.

## 2. Motivation

Today an embedding application that wants two panes reaches for raw ratatui
`Layout::default().direction(...).constraints([...])` and threads the resulting
`Rect`s by hand. That is fine but verbose, and it has no ergonomic "give pane A
60% and pane B the rest" affordance with a sane default. `SplitPane` provides:

- A named, self-documenting 2-pane abstraction with a 50/50 default.
- A single place to encode the orientation vocabulary unambiguously
  (see [§4.1](#41-orientation-semantics)).
- A clean home for the common **master/detail** layout (a selection list whose
  highlight drives a derived detail pane — see [§6.4](#64-masterdetail-a-first-class-pattern)).

> **Note — original brief not linked in-repo.** This spec paraphrases the
> requesting brief ("two knobs, both required"); the brief itself is not
> currently linked here. Reviewers separating *must* from *nice-to-have* should
> reference the original brief directly rather than trusting the paraphrase.

Non-goals (for the first cut):

- **No render wrapper in v1.** `SplitPane` ships as a geometry-only primitive;
  the optional `StatefulWidget` wrapper is deferred to a fast-follow
  (see [§8](#8-future-enhancements)).
- **No draggable / resizable divider.** The ratio is caller-controlled, not
  mouse-interactive. (Listed as a future enhancement in [§8](#8-future-enhancements).)
- **No N-way splits.** Exactly two panes. Nesting a `SplitPane` inside a
  `SplitPane` covers the recursive case.
- **No focus management or event routing.** `SplitPane` is not a `HandleEvent`
  implementer; the embedding app owns focus and the event loop.

## 3. Scope & Layering

**[DECISION — D1 — RESOLVED]** v1 ships **one layer only — the geometry core**.

| Layer | Type | Role | v1? |
| :--- | :--- | :--- | :--- |
| **Geometry core** | `SplitPane` (config struct) + `SplitPane::split(area) -> (Rect, Rect)` | Pure rectangle math. No widgets, no rendering. Always available. | **Yes** |
| **Render wrapper** | `SplitPaneWidget<A, B>` impl `StatefulWidget` | Optional convenience that renders two child widgets into the computed rects in one `render` call. | **Deferred** ([§8](#8-future-enhancements)) |

Rationale: the geometry core is the irreducible, always-correct primitive and
is trivially unit-testable without a terminal. The render wrapper is sugar for
one narrow case (two *independent* input widgets rendered together) and it
introduces generic-state plumbing
([§5.3](#53-no-heterogeneous-state-problem-in-v1)). Crucially, the dominant real
2-pane layouts — fixed-width sidebar against content, and **master/detail**
([§6.4](#64-masterdetail-a-first-class-pattern)) — either do not benefit from
the wrapper or are actively made harder by it. Most real ratatui apps already
call `render_stateful_widget` per child and only need the rects; that is the
idiomatic path and what v1 optimizes for.

Adding the wrapper later is **strictly additive** — it is a new type that calls
`split()` internally, so existing `split()`-based call sites are untouched. The
only genuinely irreversible sub-decision (its state model) is therefore
deferred until a real call site informs it (resolved direction in
[§8](#8-future-enhancements): a **named** `SplitPaneState`, not a tuple).

**[DECISION — D7 — RESOLVED]** The module home is `core`, not `components`.
Implement `biscuit-tui/lib/src/core/split_pane.rs`, add `pub mod split_pane;`
from `core/mod.rs`, and re-export `SplitPane`, `SplitDirection`, and
`SplitRatio` from both `biscuit_tui::core` and `biscuit_tui::prelude`. This
matches the existing `FrameChrome` placement for container/layout primitives.
`ResolvedAxis` remains private because `Auto` resolution is an implementation
detail in v1.

## 4. Public API (proposed)

### 4.1 Orientation semantics

**[DECISION — D2 — RESOLVED]** Confirmed by the requester:

- **`Vertical`** ⇒ one pane **on top of** the other (top / bottom),
  separated by a horizontal divider. First pane is the **top** one.
- **`Horizontal`** ⇒ panes **side-by-side** (left | right), separated by a
  vertical divider. First pane is the **left** one.

This matches ratatui's `Direction` semantics exactly (`Vertical` ⇒ top/bottom,
`Horizontal` ⇒ left/right), so there is no translation surprise for readers who
know ratatui, and it matches CSS `flex-direction` (`column` ≈ Vertical, `row` ≈
Horizontal).

A dedicated **input** enum carries those semantics (rather than overloading the
existing `Orientation` — see the note below):

```rust
/// How a [`SplitPane`] is *asked* to arrange its two child panes.
///
/// This is the caller-facing *input* vocabulary. It includes `Auto`, an
/// intent that is resolved to a concrete axis at split time
/// (see [`ResolvedAxis`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SplitDirection {
    /// Pick the direction from the area's shape at split time: a wider-
    /// than-tall area splits `Horizontal` (side-by-side); a taller-than-
    /// wide area splits `Vertical` (stacked). The default.
    #[default]
    Auto,
    /// Panes sit side-by-side (left | right), separated by a vertical
    /// divider. First pane is the left one. Maps to ratatui
    /// `Direction::Horizontal`.
    Horizontal,
    /// Panes stack one over the other (top / bottom), separated by a
    /// horizontal divider. First pane is the top one. Maps to ratatui
    /// `Direction::Vertical`.
    Vertical,
}
```

**[DECISION — D8 — RESOLVED]** The default is **`Auto`**: the split axis is
chosen from the area's shape each time `split()` runs. If the area is wider than
it is tall, the panes go side-by-side (`Horizontal`); otherwise they stack
(`Vertical`). This keeps a two-pane layout readable as the terminal is resized
without the caller re-picking a direction.

**[DECISION — D3 — RESOLVED] — `Auto` is an input-only concept, so a resolved
axis needs its own total type.** `Auto` must be turned into a concrete
`Horizontal`/`Vertical` *before* any geometry or rendering can use it. Modeling
that with a single enum forces an `unreachable!()` arm at the point of use (one
type doing two jobs: intent vs. fact). Instead, resolution produces a separate,
**private**, total enum:

```rust
/// The concrete split axis, after [`SplitDirection::Auto`] has been
/// resolved against a specific area. Total over the two real axes — there
/// is no `Auto`, so consumers never need an `unreachable!` arm.
///
/// Private to the crate today. Future concrete-axis features (a divider
/// glyph, a draggable divider — see §8) should accept a `ResolvedAxis` so
/// the compiler guarantees resolution already happened.
enum ResolvedAxis {
    Horizontal,
    Vertical,
}

impl SplitDirection {
    /// Resolves `Auto` against `area`'s shape; passes explicit directions
    /// through unchanged. Total: never returns an unresolved value.
    fn resolve(self, area: Rect) -> ResolvedAxis {
        match self {
            SplitDirection::Horizontal => ResolvedAxis::Horizontal,
            SplitDirection::Vertical   => ResolvedAxis::Vertical,
            // Auto: compare raw cells (see D9).
            SplitDirection::Auto if area.width >= area.height => ResolvedAxis::Horizontal,
            SplitDirection::Auto                              => ResolvedAxis::Vertical,
        }
    }
}
```

**[DECISION — D9 — RESOLVED]** Terminal cells are not square — they are roughly
twice as tall as they are wide — so a raw `width >= height` cell comparison
leans toward `Horizontal`. v1 compares **raw cells** (simplest, matches the
literal "wider than tall" brief). If we later want *visual* squareness, weight
height by a cell aspect factor (≈2): `area.width >= area.height * 2`. Start with
raw cells and revisit only if Auto picks feel off in practice. The `>=`
tie-break sends a perfectly square area to `Horizontal`; this tie-break is
pinned by a unit test.

> **Note on the existing `Orientation` enum.** `biscuit-tui` already exports a
> `core`/`components` `Orientation` (`Vertical` | `Horizontal`) — but its
> semantics are *content flow inside a choice list* (Vertical = one item per
> row), **not** a split axis, **and it has no `Auto` concept**. The `Auto`
> input variant is the decisive reason it cannot be reused: `SplitDirection`
> needs to carry an intent that `Orientation` cannot express. Hence a dedicated
> `SplitDirection`.

### 4.2 Split ratio

The relative space each pane gets. Default is 50/50. Representation:

```rust
/// The relative share of space given to each pane of a [`SplitPane`].
///
/// No variant ever *voluntarily* starves a pane to zero (see §5.2 for the
/// single degenerate exception). Defaults to an even split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SplitRatio {
    /// First pane takes this percentage; the second pane takes `100 - p`.
    /// Clamped into `1..=99` on construction so neither pane is ever
    /// fully starved.
    Percent(u8),
    /// First pane takes a fixed cell count (clamped to `>= 1` on
    /// construction); the second pane takes the rest (held to at least one
    /// cell where the area allows). Useful for a fixed-width sidebar
    /// against a flexible main pane.
    FirstFixed(u16),
    /// Second pane takes a fixed cell count (clamped to `>= 1` on
    /// construction); the first pane takes the rest (held to at least one
    /// cell where the area allows). Useful for a fixed-width detail panel
    /// on the right/bottom.
    SecondFixed(u16),
}

impl Default for SplitRatio {
    /// 50/50.
    fn default() -> Self {
        SplitRatio::Percent(50)
    }
}
```

**[DECISION — D4 — RESOLVED]** Keep `FirstFixed` / `SecondFixed` alongside
`Percent` in v1. A fixed-size sidebar is the most common real 2-pane layout
(file tree + content, list + detail), and the variants map cleanly onto ratatui
`Constraint::Length`. Adding variants later is non-breaking, so the cost of
including them now is low and the payoff is direct.

**[DECISION — D5 — RESOLVED] — uniform "no voluntary zero pane" invariant.**
All three variants obey one rule: **no variant collapses a pane to zero on its
own.** `Percent` is clamped to `1..=99`; the `*Fixed` variants are **clamped to
`>= 1`** on construction (so `FirstFixed(0)`/`SecondFixed(0)` cannot starve their
own pane) and pair the fixed length with `Constraint::Min(1)` for the flexible
pane (not `Min(0)`). The only case where a pane reaches zero is the genuinely
degenerate one in
[§5.2](#52-degenerate--small-area-behavior) (fixed length ≥ available area), and
that case is explicitly documented and tested. Deliberate hiding of a pane is
**not** expressed by `Percent(0)`; if it is ever needed it gets a separate,
explicit API (e.g. a collapse flag) so that "zero" is always intentional and
named.

Implement the clamping through constructors/builders rather than by trusting raw
enum values at call sites:

```rust
impl SplitRatio {
    pub fn percent(p: u8) -> Self { Self::Percent(p.clamp(1, 99)) }
    pub fn first_fixed(n: u16) -> Self { Self::FirstFixed(n.max(1)) }
    pub fn second_fixed(n: u16) -> Self { Self::SecondFixed(n.max(1)) }
}
```

`SplitPane::with_ratio` must normalize incoming enum values with the same rules,
so even direct `SplitRatio::Percent(0)` / `SplitRatio::FirstFixed(0)` values
cannot bypass the invariant. `Default` should call `SplitRatio::percent(50)`.

### 4.3 The geometry core

```rust
/// Splits a rectangle into two panes along one axis.
///
/// Geometry-only by design: it computes child rectangles and renders
/// nothing. Call [`SplitPane::split`] and render each child yourself with
/// its own `render_stateful_widget` — that is the idiomatic path
/// (see [§6.1](#61-geometry-only-the-idiomatic-path)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SplitPane {
    /// Side-by-side vs. stacked. Defaults to `Auto` (chosen from the
    /// area's shape at split time).
    pub direction: SplitDirection,
    /// Relative share for each pane. Defaults to 50/50.
    pub ratio: SplitRatio,
    /// Cells of empty space reserved *between* the two panes (a gutter).
    /// Defaults to `0`. A value of `1` is the natural home for a future
    /// divider glyph.
    pub gap: u16,
}

impl SplitPane {
    /// Even (50/50) auto-direction split with no gap.
    pub fn new() -> Self { Self::default() }

    /// Builder: set the direction.
    pub fn with_direction(self, direction: SplitDirection) -> Self { /* … */ }

    /// Builder: set the ratio.
    pub fn with_ratio(self, ratio: SplitRatio) -> Self { /* … */ }

    /// Builder: set the inter-pane gap in cells.
    pub fn with_gap(self, gap: u16) -> Self { /* … */ }

    /// Computes the two child rectangles for `area`.
    ///
    /// Returns `(first, second)` where `first` is the left pane (for a
    /// resolved `Horizontal`) or the top pane (for a resolved `Vertical`).
    /// On an odd split-axis length the **first** pane absorbs the spare
    /// cell (e.g. a 50/50 split of 9 ⇒ first 5, second 4). Both rects are
    /// guaranteed to lie within `area`; degenerate inputs collapse a pane
    /// to zero width/height rather than overflowing
    /// (see [§5.2](#52-degenerate--small-area-behavior)).
    pub fn split(&self, area: Rect) -> (Rect, Rect) { /* … */ }
}
```

The type derives `Default`, but any default or builder implementation must route
through the normalization rules in [§4.2](#42-split-ratio). Keeping the enum
fields public is acceptable for pattern matching and simple construction, but
`split()` is still responsible for defending against unnormalized values so
library callers cannot create invalid geometry by bypassing builders.

`split` is a thin wrapper over ratatui `Layout`, operating on the **resolved**
axis (no `Auto`, hence no `unreachable!`):

```rust
// Sketch — not final. `self.direction` is first resolved through
// `SplitDirection::resolve(area)` (§4.1) into a concrete `ResolvedAxis`.
let direction = match self.direction.resolve(area) {
    ResolvedAxis::Horizontal => Direction::Horizontal,
    ResolvedAxis::Vertical   => Direction::Vertical,
};
let constraints = match self.ratio {
    SplitRatio::Percent(p)     => [Constraint::Percentage(p as u16),
                                   Constraint::Percentage(100 - p as u16)],
    SplitRatio::FirstFixed(n)  => [Constraint::Length(n), Constraint::Min(1)],
    SplitRatio::SecondFixed(n) => [Constraint::Min(1), Constraint::Length(n)],
};
// `gap` becomes Layout::spacing(self.gap) (the current crate uses ratatui 0.30,
// which supports spacing):
// the fixed pane keeps exactly its `n`; the gap is removed from the total
// and the flexible pane absorbs it (see §5.1).
```

> **Implementation note — spare cell.** The documented rule is "first pane
> absorbs the spare cell." If ratatui's layout solver allocates the spare to
> the *second* pane for a given constraint set, `split()` must post-adjust the
> two rects so the documented (and tested) rule holds.

### 4.4 Public API surfaces

This feature adds library API only. It must update every live public surface
that advertises `core` exports:

- `lib/src/core/mod.rs`: add the module and re-export the three public types.
- `lib/src/prelude.rs`: re-export the three public types.
- `lib/src/lib.rs`: update module-level docs if the public surface list stays
  explicit.
- `lib/tests/public_api_names.rs`: add coverage that the names are available
  from the crate root/prelude as expected.
- `docs/components/index.md` and a new `docs/components/split_pane.md`, or a
  clearly named layout/core docs page, must describe the geometry-only usage
  pattern. If it lives outside `docs/components/`, link it from the component
  index so users looking for `FrameChrome`-style containers can find it.
  The CLI docs must not gain a `question split-pane` command.

### 4.5 No render wrapper in v1

`SplitPaneWidget<A, B>` is **not** part of the v1 surface. It is deferred to a
fast-follow with a **named** state struct; see
[§8](#8-future-enhancements). For why this is the right cut — and why it is
strictly additive to add later — see [§3](#3-scope--layering) and
[§5.3](#53-no-heterogeneous-state-problem-in-v1).

## 5. Behavior & Semantics

### 5.1 Layout math

- For `Percent(p)`: the split-axis length is divided per the ratatui solver and
  then **post-adjusted so the first pane (left/top) absorbs any spare cell** on
  an **odd** length (e.g. 9 cells at 50/50 ⇒ 5 / 4). The documented contract is
  "first absorbs the spare," regardless of the solver's internal rounding
  direction (see the implementation note in [§4.3](#43-the-geometry-core)). The
  **cross axis** (the dimension not being split) is passed through to both panes
  unchanged — both panes share `area`'s full cross-axis extent.
- For `FirstFixed(n)` / `SecondFixed(n)`: the fixed pane gets exactly `min(n,
  available)` cells; the other gets the rest, held to at least one cell where
  the area allows.
- `gap` cells are removed from the total before division and left blank between
  the panes. With a `*Fixed` ratio the **fixed pane keeps its exact `n`** and
  the **flexible pane absorbs the gap** (e.g. `FirstFixed(24)` + `gap = 1` on a
  100-cell axis ⇒ fixed 24, gap 1, flex 75). With `gap = 0` the panes are flush.
  `gap` is clamped to the available split-axis length so it can never exceed the
  area (see [§5.2](#52-degenerate--small-area-behavior)).

### 5.2 Degenerate / small-area behavior

A layout widget must never panic or overflow on a tiny terminal. Rules:

- If `area` is zero-sized, both returned rects are zero-sized.
- **[DECISION — D5 — RESOLVED] — the single zero-pane exception.** If `area` is
  too small to honor a `*Fixed` pane plus the gap, the fixed pane is clamped to
  the available cells and the flexible pane collapses to zero — no overflow, no
  panic. This is the **only** circumstance in which a pane reaches zero (the
  uniform `Min(1)` invariant in [§4.2](#42-split-ratio) prevents every
  voluntary case). It is covered by a named unit test.
- `Percent` is clamped to `1..=99` at construction so a 0/100 split cannot
  silently hide a pane the caller forgot about.
- If `gap >= area`'s split-axis length, `gap` is clamped to that length: the gap
  consumes the available space and both panes collapse to zero — no overflow, no
  panic. Covered by a named unit test.

### 5.3 No heterogeneous-state problem in v1

`ratatui::StatefulWidget` has a single associated `State`. A render wrapper
hosting **two** children with **different** state types would have to reconcile
them into one `State` — the crux that made the wrapper costly. **v1 sidesteps
this entirely by not shipping a wrapper.** With geometry-only `split()`, there
is no single associated `State` to reconcile: each child is rendered with its
**own** `&mut State` via a separate `render_stateful_widget` call
([§6.1](#61-geometry-only-the-idiomatic-path)). The per-child render is the
idiomatic path.

If a wrapper is ever added ([§8](#8-future-enhancements)), it uses a **named**
`SplitPaneState<SA, SB> { first, second }` — self-documenting at the call site
and consistent with the `FrameChromeConfig` precedent — never a positional
tuple.

### 5.4 What `SplitPane` deliberately does **not** do

- No `HandleEvent` impl, no `EventOutcome`. It is not in the input-component
  family; `run_standalone` does not drive it.
- No focus tracking. Which pane is "active" is the embedding app's concern.
- No borders. Compose with `FrameChrome` per-pane if a border is wanted, or use
  `gap` + a future divider glyph.

## 6. Usage Examples (proposed)

### 6.1 Geometry only (the idiomatic path)

```rust
use biscuit_tui::core::{SplitPane, SplitDirection, SplitRatio};

let layout = SplitPane::new()
    .with_direction(SplitDirection::Horizontal) // left | right
    .with_ratio(SplitRatio::Percent(30));       // 30% sidebar, 70% main

let (sidebar, main) = layout.split(frame.area());
frame.render_stateful_widget(ChooseOne::new(), sidebar, &mut list_state);
frame.render_stateful_widget(TextArea::new(),  main,    &mut editor_state);
```

The two rects flow into two independent render calls with no coupling between
them — different widget types, different state types, rendered in any order.

### 6.2 Fixed-width sidebar, stacked panes

```rust
// A 24-cell-wide left sidebar against a flexible main pane.
let (sidebar, main) =
    SplitPane::new().with_ratio(SplitRatio::FirstFixed(24)).split(area);

// Top status bar (3 rows) over a content pane.
let (status, content) = SplitPane::new()
    .with_direction(SplitDirection::Vertical)        // top / bottom
    .with_ratio(SplitRatio::FirstFixed(3))
    .split(area);
```

### 6.3 Nesting (the N-way story)

Nesting is just calling `split()` on a rect produced by another `split()` —
three panes, three independent render calls, zero generic plumbing:

```rust
let (sidebar, body) = SplitPane::new()
    .with_ratio(SplitRatio::FirstFixed(28))
    .split(frame.area());

let (content, status) = SplitPane::new()
    .with_direction(SplitDirection::Vertical)
    .with_ratio(SplitRatio::SecondFixed(3))          // 3-row status bar at bottom
    .split(body);

frame.render_stateful_widget(ChooseOne::new(), sidebar, &mut list_state);
frame.render_stateful_widget(TextArea::new(),  content, &mut editor_state);
frame.render_widget(status_line, status);
```

### 6.4 Master/detail (a first-class pattern)

A selection list in one pane drives a **derived** detail pane in the other:
when the **active** (highlighted) choice changes, the detail pane updates. The
detail pane holds no independent state — it is a pure function of the master's
active item, recomputed each frame. This is exactly the shape the geometry-only
path models naturally (and the shape a generic two-independent-children wrapper
would fight — see [§5.3](#53-no-heterogeneous-state-problem-in-v1)).

```rust
use std::collections::HashMap;
use biscuit_tui::core::{SplitPane, SplitDirection, SplitRatio};

// Detail content is DERIVED from the master's ACTIVE item (the highlight),
// not its submitted selection. Descriptions live in a caller-owned
// `id -> description` map (see Required Companion Work); ChoiceOption is
// not modified.
fn active_detail(master: &ChooseOneState<String>, descriptions: &HashMap<String, String>) -> String {
    match master.active_option() {                     // see Required Companion Work
        Some(opt) => descriptions
            .get(&opt.id)
            .cloned()
            .unwrap_or_else(|| format!("(no description for {})", opt.label)),
        None => "(no options)".into(),
    }
}

fn draw(
    frame: &mut Frame,
    master: &mut ChooseOneState<String>,
    descriptions: &HashMap<String, String>,
) {
    let (master_rect, detail_rect) = SplitPane::new()
        .with_direction(SplitDirection::Horizontal)    // master | detail
        .with_ratio(SplitRatio::Percent(35))
        .split(frame.area());

    frame.render_stateful_widget(ChooseOne::new(), master_rect, master);

    // Prose (biscuit-terminal) is a TerminalRenderable that produces an ANSI
    // String — it is NOT a ratatui Widget. Render it to a String fit to
    // detail_rect's width, then bridge through `ansi-to-tui` into a
    // Paragraph to paint into the rect. The exact width-fit entry point
    // (fallback_render / render / display family, plus word-wrap) is
    // confirmed against biscuit-terminal at implementation time — there is
    // no `render_in_width`.
    let detail = active_detail(master, descriptions);
    // Render Prose to an ANSI String fit to detail_rect's width (exact
    // entry point + how to obtain a width-fit Terminal pinned at impl time),
    // then bridge into a ratatui Text via ansi-to-tui.
    let ansi: String = render_prose_fit(Prose::new(detail).with_word_wrap(true), detail_rect.width);
    frame.render_widget(Paragraph::new(ansi.into_text().unwrap_or_default()), detail_rect);
}
```

> **Illustrative.** The `Prose` rendering call above shows intent, not a final
> signature — `biscuit-terminal`'s exact width-constrained rendering entry point
> (and how to obtain a `Terminal` sized to a sub-rect) is pinned during
> implementation. The load-bearing parts are: detail is **derived** from
> `active_option()` each frame, descriptions come from a **caller map**, and the
> `Prose → ansi-to-tui → Paragraph` bridge is how a `TerminalRenderable` lands
> in a ratatui `Rect`.

#### Required Companion Work (outside `SplitPane` geometry)

Master/detail is a **first-class supported pattern** for `SplitPane`, and v1's
definition of done **includes** the following additions. They live in the
`ChooseOne` component (`components/choose_one.rs`, `components/choose.rs`), not
in the `SplitPane` geometry primitive — they are an explicit, deliberate scope
expansion of this feature beyond the layout math:

1. **Public active-item accessors on `ChooseOneState`.** Today only
   `hover() -> Option<usize>` (the highlighted row, distinct from the
   submit-time `selected_value()`) and `options()` exist, forcing
   `options()[hover()?]` at call sites. Add **both**
   `active_option() -> Option<&ChoiceOption<V>>` and
   `active_value() -> Option<&V>`, keyed off `hover()`. **Semantics:** they
   return the option/value at the current highlight **as-is**, including a
   `disabled` option if the highlight rests on one — they mirror `hover()` and
   apply no disabled-filtering of their own (navigation already governs where
   the highlight may land). A `None` result means there is no active row (e.g.
   an empty option list).
   Add tests for empty options, the initial active row, active row movement,
   and the disabled-option contract using existing navigation behavior rather
   than inventing new focus rules.
2. **Per-option description data — caller-supplied map (no `ChoiceOption`
   change).** `ChoiceOption` is **not** modified (it keeps `id`, `label`,
   `value`, `disabled`, `hotkey`). The detail text comes from a caller-owned
   `id → description` map, keyed by `active_option().id`, as in the example
   above. Optionally provide a thin convenience
   `ChooseOneState::active_description(&map) -> Option<&str>` that performs the
   `active_option().id` → map lookup; this is sugar over (1), not a new data
   model.
3. **Docs note — Prose ⇄ ratatui bridge.** `Prose` (and other
   `TerminalRenderable`s, in `biscuit-terminal`) produce an ANSI `String`; they
   are **not** ratatui `Widget`s, and there is **no `render_in_width`** method —
   use the `fallback_render` / `render` / `display` family with a width-fit
   `Terminal` (and/or word-wrap). Painting the resulting string into a `Rect`
   goes through the `ansi-to-tui` crate into a `Paragraph`. `ansi-to-tui` (and
   the `biscuit-terminal` `Prose` dependency) are **example/dev-only** here,
   **not** `biscuit-tui` library dependencies.

## 7. Definition of Done & Testing Plan

### 7.1 Acceptance invariant (definition of done)

For any `area` and configuration, `split()` must satisfy the core invariant —
this is the testable bar for "correct":

- Both child rects lie entirely within `area`.
- The two child rects are non-overlapping (modulo the `gap` channel between
  them).
- Along the split axis, `first.len + gap + second.len == area.len` **where the
  area allows** (the degenerate fixed-≥-area and gap-≥-area cases in
  [§5.2](#52-degenerate--small-area-behavior) are the documented exceptions).
- Along the **cross axis**, both panes span `area`'s full cross-axis extent
  (same offset and length as `area` on that dimension).
- `split()` never panics and never overflows `area`.

### 7.2 Tests

Follow the repo's L1/L2/L3 testing taxonomy and use unit tests for pure
geometry. This feature does not need a real-terminal harness in v1 because it
adds no renderer, event loop behavior, or CLI command.

- **Geometry unit tests (L1, no terminal):**
  - 50/50 default halves an even area; odd lengths split deterministically with
    the **first** pane absorbing the spare cell (9 ⇒ 5 / 4).
  - Explicit `Horizontal` vs `Vertical` produce the expected axis split.
  - `Auto` resolves to `Horizontal` on a wide area, `Vertical` on a tall area,
    and `Horizontal` on a square area (the `>=` tie-break).
  - `FirstFixed` / `SecondFixed` honor the fixed pane exactly and flex the
    other (held to `Min(1)` where the area allows).
  - `*Fixed(0)` is clamped to `>= 1` on construction (no self-starved pane).
  - The single zero-pane exception: a `*Fixed` length ≥ available area collapses
    the flexible pane to zero — never overflows, never panics (named test).
  - Degenerate areas (`0×0`, `1×N`) never overflow and never panic; child rects
    always lie within `area`.
  - `Percent` clamping at the `0` / `100` boundaries into `1..=99`.
  - `gap` reduces total before division, lands between the panes, and (with a
    `*Fixed` ratio) is absorbed by the **flexible** pane while the fixed pane
    keeps its exact `n`.
  - Spare-cell rule survives a gap: an **odd** `gap` plus an **odd** remaining
    axis length under `Percent(50)` still gives the spare cell to the **first**
    pane.
  - `gap ≥ area`'s split-axis length is clamped to that length; both panes
    collapse to zero — never overflows, never panics (named test).
  - The acceptance invariant ([§7.1](#71-acceptance-invariant-definition-of-done))
    holds across a representative spread of areas and configs.
- **Cross-platform:** geometry is pure integer math — identical on macOS,
  Linux, Windows. No platform-specific paths.

(No render-wrapper tests in v1, since the wrapper is deferred. The companion
`ChooseOne` additions in [§6.4](#64-masterdetail-a-first-class-pattern) carry
their own unit tests in the `choose_one` module.)

### 7.3 Documentation and compatibility checks

- Update `biscuit-tui/lib/CHANGELOG.md` under `Unreleased` because this is a
  new public library API.
- Update `biscuit-tui/README.md` and `biscuit-tui/lib/README.md` if their core
  primitive lists stay explicit.
- Run the package-area verification commands after implementation:
  `just test` for unit tests, and `just lint` if the implementation changes
  public docs or exports. Do not run `cargo fmt` write-mode as part of this
  feature unless explicitly requested.

## 8. Future Enhancements

1. **Render wrapper (`SplitPaneWidget<A, B>`)** — the deferred convenience layer
   for the two-*independent*-children case. When added it will be **strictly
   additive** (a new type that calls `split()` internally; no change to existing
   `split()`-based call sites) and will use a **named**
   `SplitPaneState<SA, SB> { first, second }` state — never a positional tuple.
2. **Draggable divider** — mouse-resizable split, storing the ratio in a
   stateful `SplitPaneState` so drags persist across frames. Requires opting
   into event handling (a `HandleEvent` impl, which v1 deliberately omits) and
   would consume a `ResolvedAxis`.
3. **Divider glyph** — render a line (`│` / `─`) in the `gap` channel, themed
   via `ComponentTheme`. Consumes a `ResolvedAxis` (resolution guaranteed).
4. **Min-size constraints per pane** — a pane that refuses to shrink below N
   cells, collapsing the other first.
5. **N-way convenience** — a `SplitPane::columns(n)` / `rows(n)` helper, though
   nesting ([§6.3](#63-nesting-the-n-way-story)) already covers it.
6. **CLI surface** — `SplitPane` is a library layout primitive; it is *not*
   expected to gain a `question` subcommand (the CLI drives single-value
   prompts). Noted to set expectations.

## 9. Open Questions / Decisions

| ID | Question | Resolution |
| :--- | :--- | :--- |
| **D1** | One layer (geometry) or two (geometry + render wrapper)? | **RESOLVED:** Geometry core only in v1. Render wrapper deferred to a fast-follow ([§8](#8-future-enhancements)); adding it later is strictly additive. |
| **D2** | What do "vertical"/"horizontal" mean? | **RESOLVED:** `Vertical` ⇒ top/bottom (stacked); `Horizontal` ⇒ left/right (side-by-side). Matches ratatui `Direction`. |
| **D3** | Reuse existing `Orientation` enum? | **RESOLVED:** No — it means content-flow and has no `Auto`. A dedicated `SplitDirection` (input) plus a private total `ResolvedAxis` (resolved). |
| **D4** | Include `FirstFixed`/`SecondFixed` ratios in v1? | **RESOLVED:** Yes — fixed-sidebar is the dominant real layout; maps onto `Constraint::Length`. |
| **D5** | Allow `0`/`100` (hide a pane) or clamp? | **RESOLVED:** Uniform "no voluntary zero pane" — `Percent` clamped `1..=99`, `*Fixed` clamped `>= 1` and paired with `Min(1)`. Single documented/tested exceptions: fixed ≥ area, gap ≥ area. Deliberate hide gets a separate explicit API. |
| **D6** | How is heterogeneous child state handled? | **RESOLVED:** No wrapper in v1, so no single associated `State` — each child renders with its own `&mut State`. If a wrapper is ever added, named `SplitPaneState`, not a tuple. |
| **D7** | Module home: `core` or `components`? | **RESOLVED:** `core` (e.g. `core/split_pane.rs`, next to `frame.rs`). Public symbols `SplitPane`, `SplitDirection`, `SplitRatio` are exported from `biscuit_tui::core` **and** re-exported through the prelude (matching `FrameChrome`); `ResolvedAxis` stays crate-private. It is a container/layout primitive like `FrameChrome`, not an input component. |
| **D8** | Default direction when unspecified? | **RESOLVED:** `Auto` — wider-than-tall area splits `Horizontal`, else `Vertical`; resolved per-`split()` into a `ResolvedAxis`. |
| **D9** | Compare raw cells or weight for cell aspect ratio? | **RESOLVED:** Raw cells (`width >= height`) for v1; square ⇒ `Horizontal` via the `>=` tie-break (pinned by a test). Revisit aspect-weighting only if Auto picks feel off. |

---

*Reviewed inline and marked ready for planning and implementation. Nothing here
is implemented yet — the code blocks are API sketches, not final signatures.*

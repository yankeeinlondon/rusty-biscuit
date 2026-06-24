# SplitPane — Draft Design Specification

> **Status:** DRAFT — for review. Open architectural questions are flagged
> inline with **[DECISION]** and collected in [§9](#9-open-questions--decisions).
> **Date:** 2026-06-24
> **Area:** `biscuit-tui/lib`

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
  (see [§4.1](#41-orientation-the-naming-trap)).
- An optional `StatefulWidget` wrapper so two child widgets can be rendered in
  one call, consistent with how `FrameChrome` wraps a single child.

Non-goals (for the first cut):

- **No draggable / resizable divider.** The ratio is caller-controlled, not
  mouse-interactive. (Listed as a future enhancement in [§8](#8-future-enhancements).)
- **No N-way splits.** Exactly two panes. Nesting a `SplitPane` inside a
  `SplitPane` covers the recursive case.
- **No focus management or event routing.** `SplitPane` is not a `HandleEvent`
  implementer; the embedding app owns focus and the event loop.

## 3. Scope & Layering

**[DECISION — D1]** `SplitPane` is specified as **two layers**, mirroring the
`FrameChrome` / `FrameChromeConfig` split:

| Layer | Type | Role |
| :--- | :--- | :--- |
| **Geometry core** | `SplitPane` (config struct) + `SplitPane::split(area) -> (Rect, Rect)` | Pure rectangle math. No widgets, no rendering. Always available. |
| **Render wrapper** | `SplitPaneWidget<A, B>` impl `StatefulWidget` | Optional convenience that renders two child widgets into the computed rects in one `render` call. |

Rationale: the geometry core is the irreducible, always-correct primitive and
is trivially unit-testable without a terminal. The render wrapper is sugar for
the common case but introduces generic-state plumbing
([§5.3](#53-the-heterogeneous-state-problem)) that some callers will not want.
Keeping them separate lets a caller use just `split()` and render the children
themselves — which is what most real ratatui apps already do.

**Recommendation:** ship the geometry core first; treat the render wrapper as a
fast-follow once the core API is settled.

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

A dedicated enum carries those semantics (rather than overloading the existing
`Orientation` — see the note below):

```rust
/// How a [`SplitPane`] arranges its two child panes.
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

Resolution rule (proposed):

```rust
// Within SplitPane::split, before building the ratatui Layout:
let resolved = match self.direction {
    SplitDirection::Auto if area.width >= area.height => SplitDirection::Horizontal,
    SplitDirection::Auto                              => SplitDirection::Vertical,
    explicit                                          => explicit,
};
```

**[DECISION — D9]** Terminal cells are not square — they are roughly twice as
tall as they are wide — so a raw `width >= height` cell comparison leans toward
`Horizontal`. The proposal above compares **raw cells** (simplest, matches the
literal "wider than tall" brief). If we want *visual* squareness instead, weight
height by a cell aspect factor (≈2): `area.width >= area.height * 2`. Recommend
starting with raw cells and revisiting only if Auto picks feel off in practice.
The `>=` tie-break sends a perfectly square area to `Horizontal`.

> **Note on the existing `Orientation` enum.** `biscuit-tui` already exports a
> `core`/`components` `Orientation` (`Vertical` | `Horizontal`) — but its
> semantics are *content flow inside a choice list* (Vertical = one item per
> row), **not** a split axis. Reusing it here would overload one name with two
> meanings. **[DECISION — D3]:** introduce a separate `SplitDirection` rather
> than reuse `Orientation`. Open to feedback.

### 4.2 Split ratio

The relative space each pane gets. Default is 50/50. Proposed representation:

```rust
/// The relative share of space given to each pane of a [`SplitPane`].
///
/// Stored as the first pane's percentage of the cross-axis length
/// (0..=100). The second pane receives the remainder. Defaults to an
/// even split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SplitRatio {
    /// First pane takes this percentage (1..=99); the second pane takes
    /// `100 - p`. Clamped into `1..=99` on construction so neither pane
    /// is ever fully starved.
    Percent(u8),
    /// First pane takes a fixed cell count; the second pane takes the
    /// rest. Useful for a fixed-width sidebar against a flexible main
    /// pane.
    FirstFixed(u16),
    /// Second pane takes a fixed cell count; the first pane takes the
    /// rest. Useful for a fixed-width detail panel on the right/bottom.
    SecondFixed(u16),
}

impl Default for SplitRatio {
    /// 50/50.
    fn default() -> Self {
        SplitRatio::Percent(50)
    }
}
```

**[DECISION — D4]** The brief only strictly requires the percentage knob with a
50/50 default. The `FirstFixed` / `SecondFixed` variants are an opinionated
addition because a fixed-size sidebar is the most common real 2-pane layout
(file tree + content, list + detail). They can be cut from v1 if we want the
minimal surface — `Percent` alone satisfies the brief. **Recommend keeping
them**; they map cleanly onto ratatui `Constraint::Length` and cost little.

### 4.3 The geometry core

```rust
/// Splits a rectangle into two panes along one axis.
///
/// This is the geometry layer — it computes child rectangles and does
/// not render anything. Pair it with [`SplitPaneWidget`] to render two
/// child widgets in one call, or call [`SplitPane::split`] directly and
/// render the children yourself.
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
    /// Returns `(first, second)` where `first` is the left pane (for
    /// `Horizontal`) or the top pane (for `Vertical`). Both rects are
    /// guaranteed to lie within `area`; degenerate inputs collapse a
    /// pane to zero width/height rather than overflowing
    /// (see [§5.2](#52-degenerate--small-area-behavior)).
    pub fn split(&self, area: Rect) -> (Rect, Rect) { /* … */ }
}
```

`split` is a thin, well-defined wrapper over ratatui `Layout`:

```rust
// Sketch — not final. `self.direction` is first resolved through the
// Auto rule in §4.1, yielding a concrete Horizontal/Vertical.
let direction = match resolved {
    SplitDirection::Horizontal => Direction::Horizontal,
    SplitDirection::Vertical   => Direction::Vertical,
    SplitDirection::Auto       => unreachable!("resolved above"),
};
let constraints = match self.ratio {
    SplitRatio::Percent(p)     => [Constraint::Percentage(p as u16),
                                   Constraint::Percentage(100 - p as u16)],
    SplitRatio::FirstFixed(n)  => [Constraint::Length(n), Constraint::Min(0)],
    SplitRatio::SecondFixed(n) => [Constraint::Min(0), Constraint::Length(n)],
};
// `gap` becomes Layout::spacing(self.gap) (ratatui 0.30 supports spacing).
```

### 4.4 The render wrapper (optional layer)

```rust
/// Renders two child widgets into the panes computed by a [`SplitPane`].
///
/// Implements [`StatefulWidget`]; its `State` is the pair of the two
/// children's states. The children render into the rectangles produced
/// by [`SplitPane::split`].
pub struct SplitPaneWidget<A, B> {
    pub split: SplitPane,
    pub first: A,
    pub second: B,
}
```

See [§5.3](#53-the-heterogeneous-state-problem) for the state-pairing design,
which is the crux of this layer.

## 5. Behavior & Semantics

### 5.1 Layout math

- For `Percent(p)`: first pane gets `round(len * p / 100)` cells of the split
  axis; second gets the remainder. The **cross axis** (the dimension not being
  split) is passed through to both panes unchanged.
- For `FirstFixed(n)` / `SecondFixed(n)`: the fixed pane gets `min(n,
  available)` cells; the other gets the rest.
- `gap` cells are removed from the total before division and left blank between
  the panes. With `gap = 0` the panes are flush.

### 5.2 Degenerate / small-area behavior

A layout widget must never panic or overflow on a tiny terminal (the
[tui skill](../../.claude/skills/tui) lists "layout overflow on small
terminals" as a top gotcha). Rules:

- If `area` is zero-sized, both returned rects are zero-sized.
- If `area` is too small to honor a `*Fixed` pane plus the gap, the fixed pane
  is clamped to the available cells and the flexible pane collapses to zero —
  no overflow, no panic.
- `Percent` is clamped to `1..=99` at construction so a 0/100 split cannot
  silently hide a pane the caller forgot about. **[DECISION — D5]:** confirm
  whether `0`/`100` should be *allowed* (deliberately hide a pane) or *clamped*
  (current proposal). Recommend clamp + a separate explicit API if hiding is
  ever needed.

### 5.3 The heterogeneous-state problem

`ratatui::StatefulWidget` has a single associated `State`. `SplitPane` hosts
**two** children that almost always have **different** state types (e.g. a
`ChooseOne` on the left, a `TextArea` on the right). Options:

1. **Tuple state** — `type State = (A::State, B::State)`. `render` borrows each
   half and forwards. Simple, zero new public types. The caller stores a tuple.
   **Recommended.**
2. **Dedicated `SplitPaneState<SA, SB>` struct** — named fields `first`,
   `second`. More readable at call sites, one more public type.
3. **No wrapper at all** — drop `SplitPaneWidget`; callers use `split()` and
   render each child with its own `render_stateful_widget` call. Zero generic
   plumbing.

**[DECISION — D6]** Recommend **(1) tuple state** for the wrapper *and*
prominently document **(3)** as the idiomatic path, since most ratatui apps
already call `render_stateful_widget` per child and only need the rects. The
wrapper earns its keep only when a caller wants a single `render` call.

### 5.4 What `SplitPane` deliberately does **not** do

- No `HandleEvent` impl, no `EventOutcome`. It is not in the input-component
  family; `run_standalone` does not drive it.
- No focus tracking. Which pane is "active" is the embedding app's concern.
- No borders. Compose with `FrameChrome` per-pane if a border is wanted, or use
  `gap` + a future divider glyph.

## 6. Usage Examples (proposed)

### 6.1 Geometry only (recommended path)

```rust
use biscuit_tui::core::{SplitPane, SplitDirection, SplitRatio};

let layout = SplitPane::new()
    .with_direction(SplitDirection::Horizontal) // left | right
    .with_ratio(SplitRatio::Percent(30));       // 30% sidebar, 70% main

let (sidebar, main) = layout.split(frame.area());
frame.render_stateful_widget(ChooseOne::new(), sidebar, &mut list_state);
frame.render_stateful_widget(TextArea::new(),  main,    &mut editor_state);
```

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

### 6.3 Render wrapper (one call, tuple state)

```rust
use biscuit_tui::core::SplitPaneWidget;

let widget = SplitPaneWidget {
    split: SplitPane::new().with_ratio(SplitRatio::Percent(40)),
    first:  ChooseOne::new(),
    second: BooleanSwitch::new(),
};
frame.render_stateful_widget(widget, area, &mut (list_state, switch_state));
```

## 7. Testing Plan

Following the repo's `TestBackend` + unit-test conventions
([`rust-testing` skill](../../.claude/skills/rust-testing)):

- **Geometry unit tests (L1, no terminal):**
  - 50/50 default halves an even area; odd lengths split deterministically
    (document which pane absorbs the spare cell).
  - `Horizontal` vs `Vertical` produce the expected axis split.
  - `Auto` resolves to `Horizontal` on a wide area, `Vertical` on a tall area,
    and `Horizontal` on a square area (the `>=` tie-break).
  - `FirstFixed` / `SecondFixed` honor the fixed pane and flex the other.
  - Degenerate areas (`0×0`, `1×N`, fixed larger than area) never overflow and
    never panic; child rects always lie within `area`.
  - `Percent` clamping at the `0` / `100` boundaries.
  - `gap` reduces total before division and lands between the panes.
- **Render tests (L1, `TestBackend`):** wrapper renders both children into the
  correct cells; assert via buffer snapshot that pane content lands in the
  expected rectangle.
- **Cross-platform:** geometry is pure integer math — identical on macOS,
  Linux, Windows. No platform-specific paths.

## 8. Future Enhancements

1. **Draggable divider** — mouse-resizable split, storing the ratio in a
   `SplitPaneState` so drags persist across frames. Requires opting into event
   handling.
2. **Divider glyph** — render a line (`│` / `─`) in the `gap` channel, themed
   via `ComponentTheme`.
3. **Min-size constraints per pane** — a pane that refuses to shrink below N
   cells, collapsing the other first.
4. **N-way convenience** — a `SplitPane::columns(n)` / `rows(n)` helper, though
   nesting already covers it.
5. **CLI surface** — `SplitPane` is a library layout primitive; it is *not*
   expected to gain a `question` subcommand (the CLI drives single-value
   prompts). Noted to set expectations.

## 9. Open Questions / Decisions

| ID | Question | Proposed resolution |
| :--- | :--- | :--- |
| **D1** | One layer (geometry) or two (geometry + render wrapper)? | Two layers; ship geometry first. |
| **D2** | What do "vertical"/"horizontal" mean? | **RESOLVED:** `Vertical` ⇒ top/bottom (stacked); `Horizontal` ⇒ left/right (side-by-side). Matches ratatui `Direction`. |
| **D3** | Reuse existing `Orientation` enum? | No — it means content-flow, not split axis. New `SplitDirection`. |
| **D4** | Include `FirstFixed`/`SecondFixed` ratios in v1? | Yes (recommended); `Percent` alone satisfies the brief if we want minimal. |
| **D5** | Allow `0`/`100` (hide a pane) or clamp to `1..=99`? | Clamp; add explicit hide API only if needed. |
| **D6** | How is heterogeneous child state handled? | Tuple state for the wrapper; document the no-wrapper path as idiomatic. |
| **D7** | Module home: `core::split_pane` (next to `frame.rs`) or `components::`? | `core` — it is a layout primitive like `FrameChrome`/`frame.rs`, not an input component. Confirm. |
| **D8** | Default direction when unspecified? | **RESOLVED:** `Auto` — wider-than-tall area splits `Horizontal`, else `Vertical`; resolved per-`split()`. |
| **D9** | Compare raw cells or weight for cell aspect ratio? | Raw cells (`width >= height`) for v1; revisit if Auto picks feel off. |

---

*Authored as a draft for review. Nothing here is implemented yet; the code
blocks are API sketches, not final signatures.*

# SplitPane

`SplitPane` is a **geometry-only** layout primitive that divides one `Rect` into two child `Rect`s along a single axis. It is the first *layout* primitive in `biscuit-tui` — unlike the input components (`TextInput`, `ChooseOne`, …) it captures no value and handles no input. Its only job is spatial: take one rectangle, an orientation, and a split ratio, and produce two child rectangles via [`SplitPane::split`].

It sits in the same conceptual family as [FrameChrome](frame_chrome.md): a *container/layout* primitive that arranges other widgets, not an input component.

## Description

`SplitPane` is a plain data config struct (`direction`, `ratio`, `gap` — all `pub`). It is **not** a `StatefulWidget`, has no `HandleEvent` impl, and is not driven by `run_standalone`. Call [`SplitPane::split(area)`](#splitpane-1) to get back `(first, second)` child rectangles, then render each child yourself with its own `render_stateful_widget` call. That is the idiomatic path: the two rects flow into two independent render calls with no coupling between them — different widget types, different state types, rendered in any order.

The companion types are:

- [`SplitDirection`](#splitdirection) — the caller-facing *input* vocabulary (`Auto` / `Horizontal` / `Vertical`).
- [`SplitRatio`](#splitratio) — the relative share each pane receives (`Percent` / `FirstFixed` / `SecondFixed`).

`ResolvedAxis` (the concrete axis after `Auto` resolution) is **crate-private**: `Auto` resolution is an implementation detail in v1, and a future divider glyph or draggable divider will accept a `ResolvedAxis` so the compiler guarantees resolution already happened.

## Parameters & Defaults

### SplitPane

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `direction` | `SplitDirection` | `SplitDirection::Auto` | How the two panes are asked to arrange themselves. `Auto` is resolved from the area's shape each time `split()` runs. |
| `ratio` | `SplitRatio` | `SplitRatio::Percent(50)` | Relative share of space for each pane. Defaults to an even split. |
| `gap` | `u16` | `0` | Cells of empty space reserved *between* the two panes (a gutter). A value of `1` is the natural home for a future divider glyph. |

Builders: `SplitPane::new()` (even 50/50, `Auto`, no gap), `with_direction(dir)`, `with_ratio(ratio)` (re-normalizes its argument), `with_gap(cells)`.

### SplitDirection

The caller-facing *input* vocabulary. It includes `Auto`, an intent that is resolved to a concrete axis at split time.

| Variant | Behaviour |
|---------|-----------|
| `Auto` *(default)* | Pick the direction from the area's shape at split time: a wider-than-tall area splits `Horizontal` (side-by-side); a taller-than-wide area splits `Vertical` (stacked). A square area resolves to `Horizontal` via the `>=` tie-break. |
| `Horizontal` | Panes sit side-by-side (left \| right). The **first** pane is the left one. Maps to ratatui `Direction::Horizontal`. |
| `Vertical` | Panes stack one over the other (top / bottom). The **first** pane is the top one. Maps to ratatui `Direction::Vertical`. |

This matches ratatui's `Direction` semantics exactly (`Vertical` ⇒ top/bottom, `Horizontal` ⇒ left/right) and CSS `flex-direction` (`column` ≈ Vertical, `row` ≈ Horizontal). `Auto` compares the area's raw cells (`width >= height`), so a square area resolves to `Horizontal`.

> **Why a dedicated enum?** `biscuit-tui` already exports a `core`/`components` `Orientation` (`Vertical` \| `Horizontal`), but its semantics are *content flow inside a choice list* (Vertical = one item per row), **not** a split axis, **and it has no `Auto` concept**. The `Auto` variant is the decisive reason it cannot be reused.

### SplitRatio

The relative share of space given to each pane. No variant ever *voluntarily* starves a pane to zero — `Percent` is clamped to `1..=99` and the `*Fixed` variants to `>= 1` on construction. The only case where a pane reaches zero is the genuinely degenerate one documented under [Degenerate / small-area behavior](#degenerate--small-area-behavior).

| Variant | Construction clamp | Meaning |
|---------|--------------------|---------|
| `Percent(u8)` | `1..=99` | First pane takes this percentage; the second takes `100 - p`. |
| `FirstFixed(u16)` | `>= 1` | First pane takes a fixed cell count; the second takes the rest. Useful for a fixed-width sidebar against a flexible main pane. |
| `SecondFixed(u16)` | `>= 1` | Second pane takes a fixed cell count; the first takes the rest. Useful for a fixed-width detail panel on the right/bottom. |

Constructors enforce the clamps: `SplitRatio::percent(p)`, `SplitRatio::first_fixed(n)`, `SplitRatio::second_fixed(n)`. `SplitPane::with_ratio` (and `split()` itself) re-normalize incoming values through the same clamps, so a raw `SplitRatio::Percent(0)` struct literal cannot bypass the invariant. `SplitRatio::default()` is `Percent(50)`.

### Behavior of `split()`

`SplitPane::split(area) -> (Rect, Rect)` returns `(first, second)` where `first` is the **left** pane (for a resolved `Horizontal`) or the **top** pane (for a resolved `Vertical`).

- **Spare cell.** On an odd split-axis length the **first** pane absorbs the spare cell (a 50/50 split of 9 ⇒ first 5, second 4).
- **Cross axis.** The dimension not being split passes through to both panes unchanged — both panes share `area`'s full cross-axis extent.
- **Gap.** `gap` cells are removed from the total before division and left blank between the panes. With a `*Fixed` ratio the **fixed pane keeps its exact `n`** and the **flexible pane absorbs the gap** (e.g. `FirstFixed(24)` + `gap = 1` on a 100-cell axis ⇒ fixed 24, gap 1, flex 75).
- **Within `area`.** Both child rects always lie entirely within `area`.

### Degenerate / small-area behavior

A layout widget must never panic or overflow on a tiny terminal. `split()`:

- A zero-sized `area` yields two zero-sized rects.
- A `*Fixed` length ≥ the available axis collapses the **flexible** pane to zero (the only case a pane reaches zero). The fixed pane is clamped to the available cells.
- A `gap` ≥ the split-axis length is clamped to that length: the gap consumes the available space and **both** panes collapse to zero.

## Usage Examples

### 1. Geometry only (the idiomatic path)

```rust
use biscuit_tui::core::{SplitPane, SplitDirection, SplitRatio};

let layout = SplitPane::new()
    .with_direction(SplitDirection::Horizontal) // left | right
    .with_ratio(SplitRatio::Percent(30));       // 30% sidebar, 70% main

let (sidebar, main) = layout.split(frame.area());
frame.render_stateful_widget(ChooseOne::new(), sidebar, &mut list_state);
frame.render_stateful_widget(TextAreaInput::new(), main, &mut editor_state);
```

The two rects flow into two independent render calls with no coupling between them.

### 2. Fixed-width sidebar, stacked panes

```rust
use biscuit_tui::core::{SplitPane, SplitDirection, SplitRatio};

// A 24-cell-wide left sidebar against a flexible main pane.
let (sidebar, main) =
    SplitPane::new().with_ratio(SplitRatio::FirstFixed(24)).split(area);

// Top status bar (3 rows) over a content pane.
let (status, content) = SplitPane::new()
    .with_direction(SplitDirection::Vertical)        // top / bottom
    .with_ratio(SplitRatio::FirstFixed(3))
    .split(area);
```

### 3. Nesting (the N-way story)

Nesting is just calling `split()` on a rect produced by another `split()` — three panes, three independent render calls, zero generic plumbing:

```rust
use biscuit_tui::core::{SplitPane, SplitDirection, SplitRatio};

let (sidebar, body) = SplitPane::new()
    .with_ratio(SplitRatio::FirstFixed(28))
    .split(frame.area());

let (content, status) = SplitPane::new()
    .with_direction(SplitDirection::Vertical)
    .with_ratio(SplitRatio::SecondFixed(3))          // 3-row status bar at bottom
    .split(body);

frame.render_stateful_widget(ChooseOne::new(), sidebar, &mut list_state);
frame.render_stateful_widget(TextAreaInput::new(), content, &mut editor_state);
frame.render_widget(status_line, status);
```

### 4. Master/detail (a first-class pattern)

A selection list in one pane drives a **derived** detail pane in the other: when the **active** (highlighted) choice changes, the detail pane updates. The detail pane holds no independent state — it is a pure function of the master's active item, recomputed each frame. This is exactly the shape the geometry-only path models naturally.

The active-item accessors on `ChooseOneState` read the **highlighted** row (distinct from the submitted `selected_value()`):

- `active_option() -> Option<&ChoiceOption<V>>` — the option at the current highlight, returned **as-is** (including a `disabled` one); `None` when the list is empty.
- `active_value() -> Option<&V>` — sugar over `active_option()`.
- `active_description(&HashMap<String, String>) -> Option<&str>` — sugar over `active_option().id -> map`; `ChoiceOption` is **not** modified, the map is caller-owned.

```rust
use std::collections::HashMap;
use biscuit_tui::core::{SplitPane, SplitDirection, SplitRatio};
use biscuit_tui::{ChooseOne, ChooseOneState};

// Detail content is DERIVED from the master's ACTIVE item (the highlight),
// not its submitted selection. Descriptions live in a caller-owned
// `id -> description` map; ChoiceOption is not modified.
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

    let detail: String = master
        .active_description(descriptions)
        .map(str::to_owned)
        .unwrap_or_else(|| "(no description)".into());
    frame.render_widget(Paragraph::new(detail), detail_rect);
}
```

#### Prose ⇄ ratatui bridge (for rich detail panes)

`Prose` (and other `TerminalRenderable`s, in [`biscuit-terminal`](https://github.com/)) produce an ANSI `String`; they are **not** ratatui `Widget`s, and there is **no `render_in_width`** method. To paint rich, word-wrapped text (colors, styles, hyperlinks) into a detail `Rect`, render the `TerminalRenderable` to an ANSI `String` fit to `detail_rect.width` (using the `fallback_render` / `render` / `display` family and/or word-wrap), then bridge that string into ratatui via the [`ansi-to-tui`](https://crates.io/crates/ansi-to-tui) crate into a `Paragraph`:

```rust
// Illustrative — pin the exact width-fit entry point against biscuit-terminal
// at integration time. There is no `render_in_width`.
let ansi: String = render_prose_fit(Prose::new(detail).with_word_wrap(true), detail_rect.width);
let text = ansi_to_tui::IntoText::into_text(ansi).unwrap_or_default();
frame.render_widget(Paragraph::new(text), detail_rect);
```

`ansi-to-tui` and the `biscuit-terminal` `Prose` dependency are **example/dev-only** here — they are **not** `biscuit-tui` library dependencies. The load-bearing parts of the master/detail pattern are: detail is **derived** from `active_option()` each frame, descriptions come from a **caller map**, and the `Prose → ansi-to-tui → Paragraph` bridge is how a `TerminalRenderable` lands in a ratatui `Rect`.

## Not a CLI command

`SplitPane` is a **library layout primitive**. It is *not* exposed as a `question split-pane` subcommand — the `question` CLI drives single-value prompts, and `SplitPane` captures no value. This is deliberate and noted to set expectations; a future CLI surface is listed only as a possible enhancement, not part of v1.

v1 also ships **no render wrapper** (`SplitPaneWidget<A, B>` is deferred to a fast-follow). The idiomatic path is `split()` + two `render_stateful_widget` calls, which already covers the dominant real layouts (fixed-width sidebar, master/detail) without generic-state plumbing.

See the [CLI Reference](../cli-reference.md) for the commands that *do* exist, and [FrameChrome](frame_chrome.md) for the companion container primitive that adds borders, margins, and titles.

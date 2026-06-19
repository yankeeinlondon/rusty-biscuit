# Constraints Example Review

> These are referencing the Ratatui repo under "examples" (https://github.com/ratatui/ratatui/tree/main)


This document reviews `examples/apps/constraints`, a Ratatui example written in Rust.

Note: the provided screenshot shows the sibling `examples/apps/flex` app (`Flex Layouts`, `Legacy`, `Start`, `SpaceAround`, and spacing hotkeys). The `constraints` app shares the same broad interaction style, but its tabs are `Min`, `Max`, `Length`, `Percentage`, `Ratio`, and `Fill`, and it does not expose spacing controls.

## What the App Does

The app is an interactive visual reference for Ratatui layout constraints. It renders a set of horizontal layout examples and shows how each `Constraint` variant affects the final width allocated to a block.

Each example row is split horizontally with a different list of constraints. Every resulting region is drawn as a bordered block labeled with:

- The original constraint, such as `Length(20)`, `Percentage(75)`, or `Fill(1)`.
- The computed width in terminal cells, such as `20 px`.

The top of the app contains a tab bar. The selected tab chooses which family of constraints is demonstrated. Below the tabs, an axis line shows the current available width. The rest of the screen is the scrollable demo area.

## Components

`main`

Initializes `color_eyre` and runs the terminal application through `ratatui::run`. This delegates terminal setup and cleanup to Ratatui.

`App`

The top-level application state:

- `selected_tab`: which constraint family is currently displayed.
- `scroll_offset`: the vertical scroll position within the rendered demo content.
- `max_scroll_offset`: the maximum scroll position for the current tab.
- `state`: whether the app is running or should quit.

`AppState`

A small enum with `Running` and `Quit`. The event loop exits when the state changes to `Quit`.

`SelectedTab`

The tab model and content router. It uses `strum` derives:

- `Display` to turn variants into labels.
- `EnumIter` to build the tab list.
- `FromRepr` to move between enum variants by numeric index.

The variant order is significant because the enum discriminant is used for tab selection and next/previous navigation.

`Example`

A reusable widget that receives a list of `Constraint` values. It applies those constraints with `Layout::horizontal`, then renders one labeled illustration block per resulting area.

Ratatui widgets used directly:

- `Tabs` renders the tab strip.
- `Paragraph` renders the axis and each constraint label.
- `Block` provides title areas, padding, borders, and colored backgrounds.
- `Scrollbar` plus `ScrollbarState` renders the vertical scroll indicator.

## Interaction Flow

The run loop is intentionally simple:

1. Recompute the initial maximum scroll offset.
2. Draw the entire app with `terminal.draw`.
3. Block on one input event with `crossterm::event::read`.
4. Mutate app state based on the key.
5. Repeat until `AppState::Quit`.

Only key press events are handled via `as_key_press_event`, which avoids reacting to key release or repeat-style event variants. The handled keys are:

- `q` or `Esc`: quit.
- `l` or right arrow: next tab.
- `h` or left arrow: previous tab.
- `j` or down arrow: scroll down.
- `k` or up arrow: scroll up.
- `g` or `Home`: jump to top.
- `G` or `End`: jump to bottom.

This gives both Vim-style and arrow-key navigation. The app does not assign `+` or `-` hotkeys; those belong to the flex example shown in the screenshot.

## Tab Switching

Tab switching is implemented by treating `SelectedTab` as an ordered enum. `next` and `previous` convert the current enum variant to a `usize`, adjust it with saturating arithmetic, then convert back with `SelectedTab::from_repr`.

Two details are worth noting:

- Navigation does not wrap. Pressing previous on the first tab or next on the last tab leaves the selected tab unchanged.
- Switching tabs resets `scroll_offset` to `0` and recalculates `max_scroll_offset`, so every tab opens at the top of its examples.

The tab titles are generated from `SelectedTab::iter()`, so adding a new variant automatically adds a tab title, as long as the content routing and example count are also updated.

## Layout Management

The app uses Ratatui's immediate-mode rendering model. `App` implements `Widget`, so each draw receives the current terminal `Rect` and a mutable `Buffer`.

The top-level layout is:

- `Length(3)` for the tab bar.
- `Length(3)` for the axis.
- `Fill(0)` for the remaining demo area.

Inside each tab renderer, examples are stacked vertically with fixed-height rows based on `EXAMPLE_HEIGHT`. The current constants are:

- `ILLUSTRATION_HEIGHT = 4`
- `SPACER_HEIGHT = 0`
- `EXAMPLE_HEIGHT = 4`

Each `Example` then splits its row horizontally using exactly the constraints it is demonstrating:

```rust
let horizontal = Layout::horizontal(&self.constraints);
let blocks = area.layout_vec(&horizontal);
```

That is the central idea of the app: the explanatory UI and the behavior under inspection are the same thing. The block widths displayed on screen are read from the actual `Rect` values returned by Ratatui's layout engine.

## Scrolling and Resizing

The demo content is rendered into an off-screen `Buffer` before being copied into the visible frame. That makes vertical scrolling straightforward:

1. Create a synthetic `demo_area` tall enough for all examples plus the visible height.
2. Render the selected tab into `demo_buf`.
3. Skip `demo_area.width * scroll_offset` cells.
4. Copy exactly `area.area()` cells into the real buffer.
5. Render a scrollbar if the content is taller than the visible area or the user has scrolled.

The resizing behavior is mostly automatic. Every frame receives the current terminal area, recomputes the layouts, and redraws the examples. Because the displayed widths are derived from the freshly computed block rectangles, resizing the terminal immediately changes both the block sizes and the labels.

There is one subtle edge: `max_scroll_offset` depends only on the number of examples and fixed row height, not on the visible demo height. The app compensates by creating an off-screen buffer with `height + area.height`, which ensures the last example can still be fully visible near the bottom. This is simple and works for the fixed-height rows used here.

## Constraint Examples Covered

`Length`

Shows fixed-size constraints next to another fixed length, a minimum, and a maximum.

`Percentage`

Shows percentages against `Fill`, `Min`, `Max`, and zero-valued percentage cases.

`Ratio`

Shows equal ratios, repeated quarter ratios, mixed ratios, and a mixed row with ratio, percentage, and length.

`Fill`

Shows proportional fill weights (`Fill(1)`, `Fill(2)`, `Fill(3)`) and fill behavior around a fixed percentage.

`Min`

Shows how `Min` reserves or grows space when paired with `Percentage(100)`.

`Max`

Shows how `Max` caps space when paired with `Percentage(0)`.

## Interesting Findings

The app is more of a layout microscope than a hand-authored diagram. It does not hard-code expected widths; it asks the layout engine for rectangles and prints the actual result.

The color palette encodes Ratatui's constraint priority comments directly in the source. `Min` and `Max` use blue colors, `Length`, `Percentage`, and `Ratio` use slate colors, and `Fill` uses the darkest slate. This makes priority visually recognizable without adding extra explanatory text to the UI.

The off-screen buffer approach is a useful pattern for simple scrollable content. It avoids making each example aware of clipping or scroll state, at the cost of allocating and copying a buffer each frame.

The enum-driven tabs are compact, but they rely on keeping three things in sync: the enum variants, `get_example_count`, and the `Widget for SelectedTab` match arms. A missed update would likely show up as either unreachable content, incorrect scrolling bounds, or a missing render branch.

Compared with the flex app in the screenshot, the constraints app intentionally has fewer moving parts. There is no configurable spacing value, no `Flex` mode passed into `Layout`, and no layout cache tuning. Its focus is the behavior of individual `Constraint` variants rather than distribution of leftover space between or around items.

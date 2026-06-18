# Mouse Drawing Example Review

> These are referencing the Ratatui repo under "examples" (https://github.com/ratatui/ratatui/tree/main)

This document reviews `examples/apps/mouse-drawing`, a Ratatui example written in Rust.

## What the App Does

The app is a small terminal drawing program. It turns the whole terminal viewport into a canvas, lets the user draw colored cell-based lines with the mouse, and shows a one-cell cursor marker at the latest mouse position.

The controls are intentionally minimal:

- Click to place the first point.
- Click and drag to draw continuous lines.
- Press `Space` to switch to a random drawing color.
- Press `q` or `Esc` to quit.

There is no toolbar, file format, eraser, undo stack, or explicit canvas model. The app stores a growing list of colored terminal positions and redraws that list every frame.

## Components

`main`

Installs `color_eyre` and starts the app with `ratatui::run`. Ratatui owns the broad terminal lifecycle, while the app handles mouse capture itself.

`MouseDrawingApp`

The top-level application state:

- `should_exit`: controls the run loop.
- `mouse_position`: tracks the last mouse event position so the cursor marker can be rendered.
- `points`: the drawing data, stored as `(Position, Color)` pairs.
- `current_color`: the color assigned to newly created points.

`run`

Enables crossterm mouse capture, then repeatedly draws the UI and blocks on the next input event. When `should_exit` becomes true, it disables mouse capture and returns.

`handle_events`

Reads one crossterm event at a time. Key events are routed to `on_key_event`, mouse events are routed to `on_mouse_event`, and every other event type is ignored.

`on_key_event`

Handles only key press events. `Space` changes `current_color` to a random RGB value, while `q` and `Esc` set `should_exit`.

`on_mouse_event`

Converts the crossterm mouse coordinates into a Ratatui `Position`. Mouse down events append a point at the current color. Drag events call `draw_line`, then every mouse event updates `mouse_position`.

`draw_line`

Uses the `line_drawing` crate's Bresenham algorithm to fill all terminal cells between the last stored point and the current drag position. Each generated cell becomes a new `(Position, Color)` entry.

`render`

Draws the whole frame in immediate mode. It renders the saved points, then the mouse cursor marker, then a centered help line. The ordering matters because later widgets overwrite earlier cells.

`render_points`

Turns every saved point into a clamped 1x1 `Rect` and renders a full block symbol in that point's saved color.

`render_mouse_cursor`

Turns the latest mouse position into a clamped 1x1 `Rect` and renders a cursor marker with the current drawing color as its background.

External crates and APIs involved:

- `ratatui`: terminal setup helper, frame rendering, `Position`, `Rect`, `Size`, symbols, styles, and text widgets.
- `crossterm`: key and mouse event input plus `EnableMouseCapture` / `DisableMouseCapture`.
- `line_drawing`: Bresenham line interpolation between terminal cells.
- `rand`: random byte generation for RGB colors.
- `color_eyre`: error reporting.

## Interaction Flow

The run loop is:

1. Enable mouse capture.
2. Draw the current app state.
3. Block until crossterm returns an event.
4. Mutate app state from that event.
5. Repeat until the user quits.
6. Disable mouse capture.

This means the app is input-driven rather than tick-driven. It does not redraw on a timer; a new frame is produced before each blocking read. Mouse movement and drag events naturally drive redraws because the terminal sends mouse events while capture is enabled.

One practical consequence is that resize events are ignored as input events, but resizing still works on the next frame because every draw uses the current `frame.area()`.

## Hot Keys

The hot key handling is deliberately small, but there are a few details worth calling out.

Only `KeyEvent::is_press()` events are handled. This prevents release events, and platform-specific non-press key events, from triggering behavior. That is especially useful for examples because it keeps the semantics focused on one action per actual press.

The app uses:

- `Space`: generate a new random `Color::Rgb`.
- `q`: quit.
- `Esc`: quit.

There are no modifier combinations and no keymap abstraction. The app maps directly from `KeyCode` to state changes, which fits the size of the example. A small mismatch in the source comments is that the `on_key_event` doc comment only mentions quitting, but the function also handles color changes.

## Mouse Handling

Mouse support is the central point of the example. Crossterm mouse capture is enabled explicitly with `EnableMouseCapture` before the loop and disabled with `DisableMouseCapture` after the loop. Without capture, most terminals would not send drag and movement events to the application in the same way.

Mouse coordinates arrive as terminal cell coordinates, not pixels. The app directly maps `event.column` and `event.row` into `Position::new(column, row)`. That makes the drawing model simple: one point is one terminal cell.

The mouse event handling distinguishes only two event kinds:

- `MouseEventKind::Down(_)`: append a single point.
- `MouseEventKind::Drag(_)`: draw a line from the last stored point to the current position.

Other mouse events, including button release, scroll wheel events, and plain movement, do not draw. They still update `mouse_position`, so the cursor marker follows the latest event the terminal reports.

The surprising part is how continuous drawing is achieved. The app does not store a separate "mouse is down" flag. Instead, it relies on crossterm's `Drag` events and the last point in `points`. Every drag event draws a Bresenham line from the previous stored point to the new drag location. Because the line points are appended to the same vector, the last point advances after each drag event.

This also means a drag before any point exists does nothing. In normal use, a drag sequence starts with `Down`, so there is a starting point.

## Layout Management

The app barely uses Ratatui's layout system. There is no `Layout`, no split panes, no constraints, and no nested widget tree. The terminal frame itself is the canvas.

Rendering is managed through direct `Rect` placement:

- Each drawn cell becomes `Rect::from((position, Size::new(1, 1)))`.
- The mouse cursor uses the same 1x1 rectangle pattern.
- The help text is a centered `Line` rendered over the full `frame.area()`.

The only layout-related defensive step is `Rect::clamp(frame.area())`. This clamps each 1x1 rectangle to the current frame before rendering, which prevents out-of-bounds rendering when a saved point is outside the visible terminal after a resize.

The help line is rendered last into the full frame, so it overlays the drawing wherever its centered text lands. That is simple and visible, but it also means the top row can overwrite user drawings.

## Resizing Behavior

The drawing is stored in absolute terminal coordinates. If the terminal is resized smaller, points outside the new viewport remain in `points`, but rendering clamps each point's rectangle to the current frame. If the terminal is later resized larger, those saved points can become visible again.

There is no scaling or reflow of the drawing. A point at column 80 remains column 80 regardless of terminal size. That is the right tradeoff for this example because it demonstrates terminal-cell mouse coordinates rather than a resolution-independent drawing canvas.

The title/help line automatically recenters because it is rendered into the current `frame.area()` each frame.

## Interesting Findings

The most interesting design choice is that the app treats Ratatui widgets as single-cell drawing operations. A full block symbol styled with a foreground color becomes a pixel-like mark, and a one-cell text marker becomes the cursor. This is a compact way to demonstrate that Ratatui rendering does not need to be limited to panels, lists, and paragraphs.

The app also shows a useful bridge between event coordinates and layout primitives. Crossterm provides `column` and `row`; Ratatui's `Position` and `Rect` make those coordinates renderable with almost no translation layer.

Using Bresenham line drawing makes drag input feel continuous even though terminals send discrete mouse events. Without interpolation, fast drags would leave gaps whenever the mouse moved more than one terminal cell between events.

The current color is captured when each point is appended, not looked up at render time. Changing color affects future points and the cursor background, but existing strokes keep their original colors.

One subtle behavior is that the default `Color` value is used until the user presses `Space`. In Ratatui that means the first strokes use the default terminal foreground color, not a random color.

The app depends on a clean exit path to disable mouse capture. It uses `ratatui::run` for the main terminal lifecycle and explicitly disables mouse capture after the loop. If an error occurred after enabling mouse capture but before the disable call, mouse capture cleanup would depend on higher-level terminal restoration rather than a local guard.

## Summary

`mouse-drawing` is a focused mouse-input example. It demonstrates how to enable mouse capture, translate mouse events into Ratatui coordinates, preserve state across frames, and render directly into terminal cells. Its layout is intentionally minimal: the whole frame is a canvas, all drawing marks are 1x1 rectangles, and resizing is handled by clamping render targets to the current frame.

# Demo App Review

> These are referencing the Ratatui repo under "examples" (https://github.com/ratatui/ratatui/tree/main)

## What the app does

`examples/apps/demo` is the original Tui-rs demo carried forward as a Ratatui example. It is less a single-purpose application than a showcase of Ratatui widgets, backends, layout constraints, canvas drawing, stateful selection, and periodic updates.

The app opens a terminal UI with three tabs:

- `Tab0` shows gauges, a sparkline, selectable lists, a rotating bar chart, an optional line chart, and styled wrapped text.
- `Tab1` shows a server status table next to a canvas-rendered world map with server markers and connection lines.
- `Tab2` shows a table of Ratatui colors, with foreground and background examples.

The CLI accepts `--tick-rate` / `-t` for update cadence in milliseconds and `--unicode` / `-u` to choose richer Unicode drawing symbols where widgets support them. The default backend is crossterm, with termion and termwiz available behind feature flags.

## Main Components

### Entrypoint and Backend Selection

`src/main.rs` defines the CLI and dispatches to the active backend module. Backend choice is compile-time feature driven:

- `crossterm` is the default feature.
- `termion` is only compiled on non-Windows and only used when crossterm is not enabled.
- `termwiz` is used when neither crossterm nor termion is enabled.

This keeps the core app and UI backend-agnostic. `main` only converts the tick-rate number into a `Duration`, passes through the Unicode flag, and calls the selected backend's `run` function.

### App State

`src/app.rs` contains all durable state:

- `App`: top-level state shared by the render function and event loop.
- `TabsState`: tab titles plus the selected tab index.
- `StatefulList<T>`: a list of items plus Ratatui `ListState` for selection.
- `RandomSignal`: random `u64` generator for the sparkline.
- `SinSignal`: sine-wave point generator for chart data.
- `Signal<S>`: a sliding window over any iterator-backed data source.
- `Signals`: the two chart signals plus visible x-axis bounds.
- `Server`: static server metadata for the table and map.

The app initializes static demo data for tasks, logs, bar chart events, and servers. It also pre-fills the sparkline and sine signals so the first render has useful data instead of starting empty.

### UI Renderer

`src/ui.rs` is an immediate-mode renderer. It receives `&mut Frame` and `&mut App`, splits the current frame area into regions, and renders widgets from the current state.

The top-level render function always draws the tab bar first, then dispatches to one of three tab-specific functions based on `app.tabs.index`.

Major widgets involved:

- `Tabs` for tab navigation.
- `Gauge`, `LineGauge`, and `Sparkline` for progress and streaming data.
- `List` with `render_stateful_widget` for task and log lists.
- `BarChart` and `Chart` for static and generated time-series data.
- `Paragraph` with styled `Span`s and `Wrap`.
- `Table` for server rows and the color table.
- `Canvas`, `Map`, `Rectangle`, `Circle`, and `canvas::Line` for the world map.

### Backend Event Loops

Each backend module owns terminal setup, input polling, ticking, rendering, and cleanup:

- `src/crossterm.rs` enables raw mode, enters the alternate screen, enables mouse capture, draws each loop, polls input with a timeout, and restores the terminal afterward.
- `src/termion.rs` creates a raw alternate screen wrapped in `MouseTerminal`, then uses two threads and an `mpsc` channel: one thread sends key events and one sends periodic tick events.
- `src/termwiz.rs` uses `TermwizBackend`, polls input with a timeout, handles explicit resize events, and flushes/restores cursor visibility at exit.

The important design choice is that backend modules translate backend-specific input types into calls on `App`: `on_up`, `on_down`, `on_left`, `on_right`, `on_key`, and `on_tick`.

## How Components Interact

The runtime flow is:

1. `main` parses CLI options and chooses a backend.
2. The backend initializes terminal mode and creates `App::new(...)`.
3. The backend event loop repeatedly calls `terminal.draw(|frame| ui::render(frame, &mut app))`.
4. Input events are mapped to app methods.
5. Tick events call `app.on_tick()`.
6. `ui::render` reads the updated app state and redraws the whole screen.
7. Pressing `q` sets `app.should_quit`, causing the backend loop to return and restore terminal state.

The renderer does not own business logic. The app state does not know how it is drawn. Backend modules do not know layout details. That separation is the cleanest part of the example.

## Hot Keys

Hot keys are intentionally assigned in backend adapters rather than in the UI code:

- `h` or left arrow: previous tab.
- `l` or right arrow: next tab.
- `j` or down arrow: next task item.
- `k` or up arrow: previous task item.
- `q`: quit.
- `t`: toggle the chart panel in the first tab.

The interesting detail is that each backend repeats this mapping using its own key type. The app methods provide the common abstraction, not a central key binding table. That is simple and explicit, but it means adding a new key requires touching every backend module.

In the crossterm backend, input is filtered through `as_key_press_event()`, which avoids acting on key release or repeat variants. Termion and termwiz have different event models, so they map their own key enums directly.

## Switching Tabs

Tabs are not modeled as an enum. `TabsState` stores `Vec<&str>` titles and a numeric `index`. `next()` wraps with modulo arithmetic, and `previous()` manually wraps from `0` to the last tab.

Rendering uses that index twice:

- `Tabs::select(app.tabs.index)` highlights the current tab.
- A `match app.tabs.index` dispatches to `draw_first_tab`, `draw_second_tab`, or `draw_third_tab`.

This is compact, but the tab titles and render match arms are coupled by convention. Adding a fourth title without adding a matching render branch would produce a selectable but blank tab body.

## Layout Management

The layout is entirely declarative and recalculated on every frame from `frame.area()`. There is no stored layout state.

At the top level, the screen is split vertically:

- fixed `Length(3)` for tabs.
- `Min(0)` for the selected tab's content.

`Tab0` then uses:

- `Length(9)` for the graph summary section.
- `Min(8)` for the central chart/list area.
- `Length(7)` for the footer paragraph.

Inside the central area, the layout changes based on `app.show_chart`:

- When the chart is visible, the area is split 50/50 between list/bar widgets and the line chart.
- When the chart is hidden, the left side becomes 100%, so lists and bar chart take the full width.

`Tab1` uses a 30/70 horizontal split for server table and world map. `Tab2` uses two equal horizontal halves but only renders into the first half, leaving the second half intentionally empty.

The app uses a mix of constraint types:

- `Length` for predictable fixed-height areas.
- `Min` for the expandable content area.
- `Percentage` for proportional dashboard splits.
- `Ratio` for equal halves and table columns.

This is a good example of Ratatui's layout model: resize behavior falls naturally out of recomputing constraints against the latest terminal rectangle.

## Resizing Behavior

Most resizing is implicit. Because every draw starts from `frame.area()` and all child rectangles are derived from `Layout`, crossterm and termion do not need app-level resize state.

Termwiz is the exception. It explicitly handles `InputEvent::Resized { cols, rows }` and calls the backend's buffered terminal `resize(cols, rows)`. After that, the normal draw path picks up the new area.

The code does not add special small-screen guards. Very small terminals may squeeze fixed-height sections or leave widgets cramped, but Ratatui's layout constraints avoid manual coordinate calculations in the app code.

## Tick and Animation Model

`App::on_tick()` drives all motion:

- increments `progress`, wrapping back to `0.0` after `1.0`;
- advances sparkline data by draining old random points and appending new ones;
- advances both sine chart signals and shifts the x-axis window;
- rotates log entries by moving the last item to the front;
- rotates bar chart entries the same way.

The `Signal<S>` abstraction is a neat reusable piece: any iterator can be displayed as a sliding window by draining `tick_rate` points and extending from the source iterator.

## Novel or Surprising Findings

- The demo is backend-neutral at the app and UI layers, but not through a generalized event abstraction. Each backend directly maps its events to the same app methods.
- `t` changes the layout, not just visibility. Hiding the chart causes the list/bar area to be recomputed at 100% width.
- `Tab2` splits the screen into two equal halves and leaves the right half unused. That looks intentional for demonstration or future expansion, but it is easy to miss.
- The map uses latitude/longitude data with `Canvas`, drawing both geographic markers and connection lines. Server status affects both table row styling and marker color.
- The Unicode flag controls several visual choices: gauge rendering, bar sets, line gauge symbols, and canvas/chart markers. It is not a global Ratatui switch; each widget opts into richer symbols separately.
- The app uses `ListState` only for the task list interaction. The logs list is rendered as stateful too, but no key binding changes its selection.
- The footer includes a non-ASCII example (`10€`) and one server location is `São Paulo`, which makes this demo also exercise Unicode text rendering.

## Files Reviewed

These are all in the Ratatui repo (https://github.com/ratatui/ratatui/tree/main):

- `examples/apps/demo/Cargo.toml`
- `examples/apps/demo/README.md`
- `examples/apps/demo/src/main.rs`
- `examples/apps/demo/src/app.rs`
- `examples/apps/demo/src/ui.rs`
- `examples/apps/demo/src/crossterm.rs`
- `examples/apps/demo/src/termion.rs`
- `examples/apps/demo/src/termwiz.rs`

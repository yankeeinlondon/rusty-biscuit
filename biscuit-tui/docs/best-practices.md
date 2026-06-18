# Best Practices for Developing Ratatui Apps

> These are referencing the Ratatui repo under "examples" (https://github.com/ratatui/ratatui/tree/main)

This guide combines lessons from the reviewed Ratatui example apps
(`demo`, `constraints`, and `mouse-drawing`) with current Ratatui
application patterns. Ratatui is a lightweight immediate-mode rendering
library, not a full application framework. Good apps make that explicit:
they own their event loop, model state, update logic, and redraw the whole
UI from state on each render pass.

## Start from the Ratatui Mental Model

Build around four separate concerns:

1. Terminal setup and cleanup.
2. Event collection and translation.
3. Application state updates.
4. Rendering the current state into the current frame.

The render function should not fetch data, block on input, or decide business
behavior. It should read the current state, derive layouts from the frame area,
and render widgets. The reviewed `demo` app shows this well: backend modules
handle terminal specifics, `App` owns durable state, and `ui::render` only
draws the current state.

For small examples, direct methods like `app.on_key(...)` and `app.on_tick()`
are fine. For larger apps, introduce a message or command enum:

```rust
enum Message {
    Quit,
    NextTab,
    PreviousTab,
    ScrollDown,
    DataLoaded(Result<Data, Error>),
    Resize,
    Tick,
}

fn update(app: &mut App, message: Message) {
    match message {
        Message::Quit => app.running = false,
        Message::NextTab => app.tabs.next(),
        Message::PreviousTab => app.tabs.previous(),
        Message::ScrollDown => app.scroll.down(),
        Message::DataLoaded(result) => app.data = result.ok(),
        Message::Resize => app.clamp_view_state(),
        Message::Tick => app.on_tick(),
    }
}
```

This keeps input bindings, background work, and UI behavior from tangling as
the app grows.

## Use a Robust Terminal Lifecycle

Prefer `ratatui::run` for new fullscreen applications. It initializes a
terminal with common defaults and restores it on normal exit and panic.

```rust
fn main() -> std::io::Result<()> {
    ratatui::run(|mut terminal| {
        let mut app = App::default();

        while app.is_running() {
            terminal.draw(|frame| draw(frame, &mut app))?;
            handle_next_event(&mut app)?;
        }

        Ok(())
    })
}
```

Use `ratatui::init()` plus `ratatui::restore()` only when you need more control,
such as custom panic hooks, embedded terminal use, or manual backend setup.
If you enable extra terminal modes yourself, clean them up explicitly. For
example, the `mouse-drawing` app enables Crossterm mouse capture separately
and disables it after the app loop. In production code, prefer a small guard
type for this kind of extra mode so cleanup still happens on early returns.

## Draw Once Per Tick

Render the complete UI inside one `terminal.draw(...)` call per loop iteration.
Ratatui already diffs buffers and writes only the changes to the terminal.
Multiple draw calls in one logical tick can cause flicker, inconsistent state,
and harder-to-reason-about rendering.

The normal flow is:

1. Draw the current state.
2. Read or poll for events.
3. Update state from events.
4. Apply periodic tick work if needed.
5. Repeat.

Input-driven apps, like `mouse-drawing`, can block on the next event. Dashboards,
animations, and networked apps should poll with a timeout so ticks and
background updates continue even when the user is not pressing keys.

Typical tick rates:

- Static or mostly input-driven apps: redraw only on events.
- Dashboards: 10 to 20 Hz is usually enough.
- Focused animation: 30 to 60 Hz only where needed.

## Keep State Durable and Rendering Disposable

Store user-visible state in your app model, not in widgets created during
rendering. The render pass is disposable and should be reconstructable from
state at any time.

Good state to keep:

- selected tab or route;
- focused component;
- selected row/list index;
- scroll offsets;
- text input contents;
- current color/tool/mode;
- cached expensive render inputs, such as parsed Markdown;
- background task results and loading/error states.

Bad state to keep:

- rectangles from the previous frame, unless used only for mouse hit testing
  and recomputed every draw;
- widgets that are cheap to rebuild;
- hard-coded widths produced by an earlier terminal size.

For Ratatui stateful widgets, keep the widget state in `App`:

```rust
struct App {
    items: Vec<Item>,
    list_state: ratatui::widgets::ListState,
}
```

Mutate `list_state` in update logic, then pass it to
`render_stateful_widget` during drawing.

## Model Navigation Explicitly

Avoid loosely coupling visible tabs to numeric indexes. The reviewed `demo`
uses tab titles plus a numeric index, which is compact but makes it possible
to add a title without adding a render branch. The `constraints` app uses an
enum with derived iteration, which is better for keeping tabs discoverable and
typed.

For non-trivial apps, prefer an enum:

```rust
enum Screen {
    Dashboard,
    Logs,
    Settings,
}
```

Then centralize routing:

```rust
match app.screen {
    Screen::Dashboard => draw_dashboard(frame, app, area),
    Screen::Logs => draw_logs(frame, app, area),
    Screen::Settings => draw_settings(frame, app, area),
}
```

Define whether tab navigation wraps or clamps, and apply that behavior
consistently. Reset scroll or selection on screen changes only when that is the
least surprising user experience.

## Treat Layout as a First-Class Design Tool

Recompute layout from `frame.area()` on every render. Do not cache layout
rectangles across frames. The `demo` and `constraints` apps both resize well
because all rectangles are derived from the current frame.

Use constraints intentionally:

- `Length(n)` for fixed-height headers, footers, status bars, and known control
  rows.
- `Min(n)` for main content that should absorb remaining space.
- `Fill(weight)` for proportional leftover space.
- `Percentage(p)` and `Ratio(a, b)` for broad, responsive splits.
- `Max(n)` for panels that should stop growing after they reach useful width.

A common base layout:

```rust
use ratatui::layout::{Constraint, Layout};

let [header, body, footer] = Layout::vertical([
    Constraint::Length(1),
    Constraint::Min(0),
    Constraint::Length(1),
])
.areas(frame.area());
```

Prefer deriving child layouts from parent areas rather than hand-calculating
coordinates. When doing direct cell rendering, as in `mouse-drawing`, convert
positions to `Rect`s and clamp them to the frame before rendering.

## Design for Small Terminals and Resize

Terminals resize constantly. Users also run TUIs in split panes, SSH sessions,
and narrow editor terminals.

Practical rules:

- Start every draw from `frame.area()`.
- Use `Min(0)` or `Fill(1)` for regions that may shrink heavily.
- Make fixed-height regions earn their space.
- Provide compact fallbacks for narrow widths.
- Clamp direct-rendered `Rect`s with the current area.
- Avoid assuming labels fit. Truncate, wrap, or hide secondary details.
- Test at small sizes, such as 80x24 and narrower.

When rendering custom widgets, never write outside the buffer area:

```rust
fn render(area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
    let area = area.intersection(buf.area);
    // render only inside `area`
}
```

## Make Scrolling a Deliberate Pattern

Simple scrollable content can be implemented with a stored `scroll_y: u16` and
widgets that support `.scroll(...)`, such as `Paragraph`.

For custom fixed-row content, the `constraints` app demonstrates a useful
pattern: render content to an off-screen buffer, copy the visible slice into
the real buffer, and draw a scrollbar. This keeps each row widget unaware of
scrolling. It is appropriate when content is moderate and fixed-height.

For large or dynamic content, prefer virtualized rendering:

- calculate the visible row range from `scroll_offset` and viewport height;
- render only visible rows;
- keep scrollbar state derived from total item count and viewport height.

Always clamp scroll offsets after resize or content changes. When switching
tabs or filters, decide whether scroll should reset to top or preserve position.

## Handle Input at the Edge

Use an adapter layer to translate backend-specific input into app messages.
This keeps the rest of the app independent from Crossterm, Termion, or Termwiz
event types.

For Crossterm, handle only key press events:

```rust
use crossterm::event::{Event, KeyCode, KeyEventKind};

fn event_to_message(event: Event) -> Option<Message> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(Message::Quit),
            KeyCode::Right | KeyCode::Char('l') => Some(Message::NextTab),
            KeyCode::Left | KeyCode::Char('h') => Some(Message::PreviousTab),
            _ => None,
        },
        Event::Resize(_, _) => Some(Message::Resize),
        _ => None,
    }
}
```

The reviewed examples consistently filter press events, which avoids duplicate
actions on platforms that emit release or repeat variants.

Use both mnemonic keys and common terminal conventions where they make sense:

- `q` and `Esc` for quitting or leaving a screen;
- arrows for navigation;
- Vim-style `h`, `j`, `k`, `l` for users who expect them;
- `g` and `G` or `Home` and `End` for scroll bounds;
- `Tab` and `Shift+Tab` for focus traversal when there are multiple controls.

For larger apps, centralize key bindings so help text, tests, and behavior do
not diverge.

## Mouse Support Needs Extra Care

Mouse events are terminal-cell coordinates, not pixels. Map them to
`Position` or `Rect` and use the same layout areas used for rendering to decide
which component was clicked.

Best practices:

- Enable mouse capture only when needed.
- Disable mouse capture on exit.
- Use layout rectangles for hit testing.
- Distinguish down, drag, up, scroll, and movement where behavior differs.
- Preserve enough state for drag interactions, such as the active button,
  origin, or last point.
- Clamp render targets after resize.

For continuous drawing or dragging, interpolate between terminal cells if
missing intermediate cells would look broken. The `mouse-drawing` app uses
Bresenham line interpolation so fast drags do not leave gaps.

## Keep Rendering Predictable

Ratatui draws in order: later widgets overwrite earlier cells. Use that
intentionally.

Common layering order:

1. Background or base content.
2. Main panels and widgets.
3. Selection or cursor indicators.
4. Overlays, popups, and help.

The `mouse-drawing` app renders saved points, then the cursor, then a centered
help line. That makes the help visible, but also means it overwrites drawings.
For production apps, put persistent help in a reserved footer or make overlays
modal and dismissible.

Do not render explanatory text over critical data unless the user can hide it.

## Use Styling as Information, Not Decoration

Terminals vary by theme, palette, font, and Unicode support. Use style to
encode state, but do not make color the only carrier of meaning.

Good uses of style:

- selected, focused, disabled, error, warning, and success states;
- status severity in tables and logs;
- active tab or active mode;
- subtle grouping between regions.

Practical rules:

- Provide text, symbols, or position in addition to color.
- Prefer theme-friendly colors over exact RGB unless the app domain requires
  specific colors.
- Test with light and dark terminal themes.
- Use Unicode symbols only when they improve clarity.
- Offer ASCII or simpler symbols when broad compatibility matters.

The `demo` app's Unicode flag is a useful model: richer symbols are opt-in per
widget rather than assumed globally.

## Do Not Block the UI on Slow Work

Ratatui can be used from synchronous or async programs, but the render loop
should stay responsive. Slow file IO, network calls, subprocesses, and database
queries should run outside the draw path.

Common architecture:

- UI loop owns `App` state.
- Background tasks or threads do slow work.
- Results arrive through channels.
- The UI loop drains messages on each tick and updates state.

For async applications, keep ownership simple. You can run a Tokio runtime for
background work and still have a straightforward UI loop that receives messages
through `mpsc` channels.

## Cache Expensive View Data

Rebuilding widgets is normal. Recomputing expensive data every frame is not.

Cache:

- parsed Markdown or syntax highlighted text;
- sorted or filtered table rows;
- measured column widths for large tables;
- expensive charts or aggregates;
- loaded images or generated cell art.

Invalidate caches when their inputs change. Keep the cache in app state, not in
the render function. The rendering code should remain a cheap projection from
state to widgets.

## Choose Backend and Dependency Versions Deliberately

Most applications should use the default Crossterm backend unless they have a
specific portability or terminal-integration reason to choose another backend.

Avoid multiple semver-incompatible Crossterm versions in the same dependency
graph. Ratatui 0.30+ supports Crossterm compatibility feature flags:

```toml
[dependencies]
ratatui = { version = "0.30", features = ["crossterm_0_29"] }
crossterm = "0.29"
```

Use `cargo tree -p crossterm` when event types appear incompatible or raw mode
cleanup behaves strangely.

If you are writing reusable widgets rather than an application, consider
depending on `ratatui-core` so your widget is less coupled to backend and
top-level crate changes.

## Test TUI Behavior

TUI tests should focus on state transitions and rendering invariants.

Recommended coverage:

- pure update tests for key messages, tab switching, scrolling, and quitting;
- snapshot tests for important widgets or screens using a fixed `Buffer`;
- resize tests at small and normal terminal sizes;
- scroll bounds after content changes;
- input translation tests for key bindings;
- custom widget tests that prove rendering stays inside the provided area.

Keep most logic out of `draw` so it can be tested without a terminal.

## Use Ecosystem Widgets Where They Fit

Do not hand-roll complex interaction primitives unless the app's behavior is
unusual.

Useful crates and patterns:

- `tui-textarea` for multiline text editing.
- `edtui` for editor-like widgets with modes and richer editing behavior.
- `tui-markdown` for rendering Markdown help or documentation panes.
- `tui-logger` for in-app logs.
- `tachyonfx` for targeted transitions and effects.
- Higher-level frameworks such as `tui-realm` when the app needs component
  focus, event routing, and stronger structure.

Use effects and heavy widgets sparingly. A fast, legible terminal app is better
than one that spends its frame budget on decorative motion.

## A Practical Project Structure

For a small to medium app:

```text
src/
  main.rs        # CLI, terminal lifecycle, error setup
  app.rs         # App state and update methods
  event.rs       # backend input to Message translation
  ui.rs          # top-level draw function and screen routing
  screens/       # larger screen-specific render/update helpers
  widgets/       # reusable custom widgets
```

For examples and tiny tools, fewer files are fine. The important boundary is
conceptual: setup, input, update, and render should be easy to identify.

## Checklist

Before calling a Ratatui app production-ready, verify:

- Terminal cleanup works on quit, error, and panic.
- The app uses one draw call per logical frame.
- Rendering starts from `frame.area()` and handles resize.
- Crossterm key handling filters press events.
- Stateful widget state lives in `App`.
- Scroll offsets and selections are clamped after content or size changes.
- Input bindings are centralized or documented in one place.
- Mouse capture, if enabled, is disabled on exit.
- Slow work is outside the draw path.
- Expensive render inputs are cached.
- The app is usable at 80x24 and in narrow panes.
- Color is not the only indicator of important state.
- Dependency versions do not pull in conflicting Crossterm majors.

## Sources Used

- Local reviews: `./example-apps/demo.md`, `./example-apps/constraints.md`, and
  `./example-apps/mouse-drawing.md`.
- Ratatui API documentation for `ratatui::run`, `Terminal::draw`, full redraws,
  frame areas, and resize behavior: <https://docs.rs/ratatui/latest/ratatui/>
- Ratatui application pattern documentation for model/update/message structure:
  <https://ratatui.rs/concepts/application-patterns/the-elm-architecture/>
- Ratatui backend documentation for Crossterm compatibility and backend
  modularization: <https://ratatui.rs/concepts/backends/>
- Ratatui 0.30 release notes for `ratatui::run`, modular crates, layout, and
  backend changes: <https://ratatui.rs/highlights/v030/>

---
name: tui-ratatui-best-practices
description: Building production-ready Ratatui applications
---


# Ratatui Application Best Practices

**Use this skill when:**

- Structuring a non-trivial Ratatui app (beyond a single-file demo)
- Deciding on event loop, state, and rendering boundaries
- Handling resize, scrolling, mouse, and async without blocking the UI
- Reviewing a TUI for production readiness

Ratatui is a lightweight **immediate-mode** rendering library, not an application
framework. Good apps make that explicit: they own their event loop, model their
own state, run their own update logic, and redraw the whole UI from state on each
render pass.

Distilled from reviews of the upstream `demo`, `constraints`, and `mouse-drawing`
examples — see `biscuit-tui/docs/example-apps/` and `biscuit-tui/docs/best-practices.md`
for the long-form source.

## The Four Concerns

Keep these separate:

1. **Terminal setup and cleanup**
2. **Event collection and translation**
3. **Application state updates**
4. **Rendering current state into the current frame**

The render function should not fetch data, block on input, or decide business
behavior. It reads state, derives layouts from the frame area, and draws widgets.

For small apps, direct methods (`app.on_key(...)`, `app.on_tick()`) are fine. As
an app grows, adopt the Elm-style **message/update** split so input bindings,
background work, and UI behavior stop tangling:

```rust
enum Message {
    Quit,
    NextTab,
    ScrollDown,
    DataLoaded(Result<Data, Error>),
    Resize,
    Tick,
}

fn update(app: &mut App, message: Message) {
    match message {
        Message::Quit => app.running = false,
        Message::NextTab => app.tabs.next(),
        Message::ScrollDown => app.scroll.down(),
        Message::DataLoaded(result) => app.data = result.ok(),
        Message::Resize => app.clamp_view_state(),
        Message::Tick => app.on_tick(),
    }
}
```

## Terminal Lifecycle

Prefer `ratatui::run` for new fullscreen apps (Ratatui 0.30+). It applies common
defaults and restores the terminal on normal exit **and panic**:

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

Use `ratatui::init()` + `ratatui::restore()` only when you need more control
(custom panic hooks, embedded terminals, manual backend setup). If you enable
extra modes yourself (e.g. mouse capture), clean them up explicitly — wrap them
in a small guard type so cleanup still runs on early returns.

> On Ratatui ≤ 0.29 (the version `biscuit-tui` pins), `ratatui::run` is
> unavailable — use the manual `enable_raw_mode` / `EnterAlternateScreen` setup
> with a panic hook (see [ratatui-architecture.md](./ratatui-architecture.md)).
> `biscuit-tui`'s own runner (`run_standalone`) already encapsulates this.

## Draw Once Per Tick

Render the complete UI inside one `terminal.draw(...)` call per loop iteration.
Ratatui diffs buffers and writes only changed cells; multiple draws per logical
tick cause flicker and inconsistent state. The flow:

1. Draw current state → 2. Poll/read events → 3. Update state → 4. Periodic tick
work → repeat.

Input-driven apps may **block** on the next event. Dashboards, animations, and
networked apps should **poll with a timeout** so ticks and background updates keep
flowing. Typical rates:

| App kind | Cadence |
|---|---|
| Static / input-driven | Redraw only on events |
| Dashboards | 10–20 Hz |
| Focused animation | 30–60 Hz, only where needed |

## State Durable, Rendering Disposable

Store user-visible state in the app model, not in widgets built during rendering.
The render pass must be reconstructable from state at any time.

| Keep in `App` | Do **not** keep |
|---|---|
| selected tab/route, focused component | rectangles from the previous frame¹ |
| list/row index, scroll offsets | widgets that are cheap to rebuild |
| text input contents, current mode/tool | hard-coded widths from an earlier size |
| cached expensive inputs (parsed Markdown) | |
| background task results, loading/error states | |

¹ Exception: rects kept *only* for mouse hit-testing and recomputed every draw.

For stateful widgets, keep the widget state in `App` (e.g. a
`ratatui::widgets::ListState`), mutate it in update logic, then pass it to
`render_stateful_widget` while drawing.

## Model Navigation Explicitly

Prefer a typed enum with derived iteration over loose numeric tab indexes — it
keeps screens discoverable and forces a render branch per variant:

```rust
enum Screen { Dashboard, Logs, Settings }

match app.screen {
    Screen::Dashboard => draw_dashboard(frame, app, area),
    Screen::Logs => draw_logs(frame, app, area),
    Screen::Settings => draw_settings(frame, app, area),
}
```

Decide whether navigation **wraps or clamps** and apply it consistently. Reset
scroll/selection on screen change only when that is the least surprising behavior.

## Layout from `frame.area()` Every Frame

Recompute layout from `frame.area()` on every render — never cache layout rects
across frames. Use constraints intentionally:

| Constraint | Use for |
|---|---|
| `Length(n)` | fixed headers, footers, status bars, control rows |
| `Min(n)` | main content that absorbs remaining space |
| `Fill(weight)` | proportional leftover space |
| `Percentage(p)` / `Ratio(a,b)` | broad responsive splits |
| `Max(n)` | panels that stop growing past a useful width |

```rust
use ratatui::layout::{Constraint, Layout};

let [header, body, footer] = Layout::vertical([
    Constraint::Length(1),
    Constraint::Min(0),
    Constraint::Length(1),
]).areas(frame.area());
```

Derive child layouts from parent areas; don't hand-calculate coordinates. See
[layout-system.md](./layout-system.md) for nested layouts and macros.

## Design for Small Terminals and Resize

Terminals resize constantly (split panes, SSH, narrow editor terminals). Rules:

- Start every draw from `frame.area()`.
- Use `Min(0)` / `Fill(1)` for regions that may shrink heavily.
- Make fixed-height regions earn their space; provide compact narrow-width fallbacks.
- Truncate, wrap, or hide secondary details — never assume labels fit.
- Test at small sizes (80×24 and narrower).

When rendering custom widgets, never write outside the buffer area:

```rust
fn render(area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
    let area = area.intersection(buf.area);
    // render only inside `area`
}
```

## Scrolling, Input, Mouse

- **Scrolling** — store a `scroll_y: u16` for simple cases; virtualize (compute the
  visible row range, render only those) for large/dynamic content. Always clamp
  offsets after resize or content change. Deep dive: [scrolling.md](./scrolling.md).
- **Input at the edge** — translate backend events into app `Message`s in an
  adapter layer, keeping the rest of the app independent of crossterm/termion. For
  crossterm, handle only `KeyEventKind::Press` to avoid duplicate actions from
  release/repeat variants. Centralize key bindings so help text, tests, and
  behavior stay in sync. Deep dive: [event-handling.md](./event-handling.md).
- **Mouse** — events are terminal-cell coordinates, not pixels. Enable capture
  only when needed and disable on exit; hit-test against the same layout rects used
  for rendering. Preserve drag state (button, origin, last point) and interpolate
  between cells (Bresenham) for continuous drawing.

## Predictable Layering

Ratatui draws in order — later widgets overwrite earlier cells. Layer
intentionally: background → main panels → selection/cursor → overlays/popups/help.
Don't render explanatory text over critical data unless the user can dismiss it;
put persistent help in a reserved footer.

## Style as Information, Not Decoration

Color must not be the only carrier of meaning (themes, palettes, and Unicode
support vary). Pair color with text, symbols, or position; prefer theme-friendly
colors over exact RGB; test light and dark themes; make rich Unicode opt-in.
Deep dive: [styling.md](./styling.md).

## Never Block the UI on Slow Work

Keep file IO, network, subprocesses, and DB queries out of the draw path:

- UI loop owns `App` state.
- Background tasks/threads do slow work and send results over channels.
- The UI loop drains messages each tick and updates state.

A Tokio runtime for background work pairs fine with a straightforward
`mpsc`-driven UI loop. Deep dive: [async-integration.md](./async-integration.md).

## Cache Expensive View Data

Rebuilding widgets is normal; recomputing expensive *data* every frame is not.
Cache parsed/highlighted Markdown, sorted/filtered rows, measured column widths,
charts/aggregates, and loaded images **in app state** — invalidate when inputs
change. Rendering stays a cheap projection from state to widgets.

## Backend and Dependency Versions

Use the default crossterm backend unless you have a specific portability reason.
Avoid multiple semver-incompatible crossterm majors in one graph — Ratatui 0.30+
offers compat feature flags:

```toml
[dependencies]
ratatui = { version = "0.30", features = ["crossterm_0_29"] }
crossterm = "0.29"
```

Use `cargo tree -p crossterm` when event types or raw-mode cleanup behave
strangely. For reusable *widgets* (not apps), depend on `ratatui-core` to stay
decoupled from backend/top-level changes.

## Testing

Keep logic out of `draw` so it tests without a terminal. Cover:

- pure `update` tests for key messages, tab switching, scrolling, quitting;
- snapshot tests of key widgets/screens against a fixed `Buffer`;
- resize tests at small and normal sizes;
- scroll-bounds tests after content changes;
- input-translation tests for key bindings;
- custom-widget tests proving rendering stays inside the provided area.

Deep dives: [testing-a-tui.md](./testing-a-tui.md), [testing-tools.md](./testing-tools.md).

## Ecosystem Widgets

Don't hand-roll complex primitives unless behavior is unusual: `tui-textarea`
(multiline edit), `edtui` (modal editor), `tui-markdown` (help/docs panes),
`tui-logger` (in-app logs), `tachyonfx` (transitions), `tui-realm` (component
focus/routing). Use effects sparingly — a fast, legible app beats decorative
motion. See [widget-collections.md](./widget-collections.md).

## Practical Project Structure

```text
src/
  main.rs        # CLI, terminal lifecycle, error setup
  app.rs         # App state and update methods
  event.rs       # backend input → Message translation
  ui.rs          # top-level draw + screen routing
  screens/       # screen-specific render/update helpers
  widgets/       # reusable custom widgets
```

The important boundary is conceptual: setup, input, update, and render should be
easy to identify.

## Production Readiness Checklist

- Terminal cleanup works on quit, error, and panic.
- One draw call per logical frame.
- Rendering starts from `frame.area()` and handles resize.
- Crossterm key handling filters press events.
- Stateful widget state lives in `App`.
- Scroll offsets and selections clamped after content/size changes.
- Key bindings centralized or documented in one place.
- Mouse capture, if enabled, is disabled on exit.
- Slow work is outside the draw path.
- Expensive render inputs are cached.
- Usable at 80×24 and in narrow panes.
- Color is not the only indicator of important state.
- No conflicting crossterm majors in the dependency graph.

## Sources

- Example-app reviews: `biscuit-tui/docs/example-apps/{demo,constraints,mouse-drawing}.md`
- Long-form guide: `biscuit-tui/docs/best-practices.md`
- Ratatui docs: <https://docs.rs/ratatui/latest/ratatui/>,
  the Elm architecture pattern <https://ratatui.rs/concepts/application-patterns/the-elm-architecture/>,
  and the 0.30 highlights <https://ratatui.rs/highlights/v030/>

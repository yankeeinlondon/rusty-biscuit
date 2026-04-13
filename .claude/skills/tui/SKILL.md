---
name: tui
description: Expert knowledge for building and designing terminal user interfaces (TUIs) covering design systems (layout paradigms, color palettes, keyboard navigation, data visualization), framework-agnostic best practices, real-world app pattern analysis, Ratatui (Rust) with immediate-mode rendering and constraint-based layouts, and Bubble Tea (Go) with Elm architecture and Charm.sh ecosystem
last_updated: 2025-12-24T12:00:00Z
hash: 984cf841655dd773
---

# Terminal User Interface Development

This skill covers TUI development across frameworks: **Ratatui** (Rust, immediate-mode) and **Bubble Tea** (Go, Elm architecture). Most detail files focus on Ratatui since it's the primary framework in this monorepo.

## TUI Design & App Patterns

Before writing code, start with design. The [TUI Design System](./tui-design-skill.md) covers:

- **Layout paradigms** — persistent multi-panel, Miller columns, drill-down stack, widget dashboard, IDE three-panel, overlay/popup
- **Interaction models** — keyboard layers, focus management, search/filtering, help systems, confirmation dialogs
- **Color design** — 16/256/truecolor tiers, semantic color slots, theme architecture, accessibility (WCAG AA)
- **Data visualization** — block elements, braille graphics, sparklines, gauges, spinners
- **Animation & motion** — flicker-free rendering, synchronized output
- **Design principles & anti-patterns** — the 7 principles and 10 ranked anti-patterns

Supporting resources:
- [App Patterns Gallery](./resources/app-patterns.md) — design analysis of lazygit, k9s, yazi, btop, zellij, and more
- [Visual Catalog](./resources/visual-catalog.md) — box-drawing, block elements, braille, status indicators, tree characters

## Choosing a Framework

| | **Ratatui** (Rust) | **Bubble Tea** (Go) |
|---|---|---|
| **Architecture** | Immediate-mode rendering | Elm (Model-Update-View) |
| **State** | Mutable app struct, redraw each frame | Immutable model, return new model |
| **Async** | Tokio channels + `try_recv()` | Commands (Cmd) return messages |
| **Styling** | `Style`, `Color`, `Modifier` on widgets | Lip Gloss (separate styling library) |
| **Widgets** | Built-in + community crates | Bubbles component library |
| **Rendering** | Buffer diffing (only changed cells) | Full string output each frame |
| **Best for** | High-performance, pixel-precise UIs | Rapid prototyping, composable UIs |

## Cross-Framework Principles

- **Separate state from view** -- store data and UI state, create widgets/views on the fly
- **Never block the render loop** -- offload work to background tasks, use non-blocking checks
- **Always restore terminal state** -- panic hooks (Rust) or deferred cleanup (Go)
- **Limit redraws** -- poll with timeout or only redraw on state changes
- **Consistent keybindings** -- `q`/`Ctrl+C` quit, arrows/hjkl navigate, `?` help, `/` search

See [TUI Best Practices](./tui-best-practices.md) for accessibility, performance, and design guidelines.

## Ratatui Quick Start

```rust
use ratatui::{backend::CrosstermBackend, widgets::{Block, Borders, Paragraph}, Terminal};
use crossterm::{event::{self, Event, KeyCode}, execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    loop {
        terminal.draw(|f| {
            let p = Paragraph::new("Hello Ratatui!")
                .block(Block::default().borders(Borders::ALL).title("Demo"));
            f.render_widget(p, f.area());
        })?;
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') { break; }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
```

See [Ratatui Architecture](./ratatui-architecture.md) for the full app pattern with event loop and state management.

## Bubble Tea Quick Start

```go
type model struct{ count int }

func (m model) Init() tea.Cmd                           { return nil }
func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    if key, ok := msg.(tea.KeyMsg); ok {
        switch key.String() {
        case "q": return m, tea.Quit
        case "up": m.count++
        case "down": m.count--
        }
    }
    return m, nil
}
func (m model) View() string { return fmt.Sprintf("Count: %d\n(q to quit)", m.count) }

func main() { tea.NewProgram(model{}).Run() }
```

See [Bubble Tea Architecture](./bubble-tea-architecture.md) for commands, async, navigation, and testing patterns.

## Common Gotchas

| Problem | Cause | Fix |
|---|---|---|
| 100% CPU | Redrawing without poll timeout | Use `event::poll(Duration::from_millis(16))` |
| Terminal corruption on panic | Raw mode not restored | Set panic hook to call `disable_raw_mode()` + `LeaveAlternateScreen` |
| Widget state lost each frame | Storing widgets instead of data | Only store data + state structs in App; create widgets in `draw` |
| Layout overflow on small terminals | No minimum size guard | Check `area.width < 10` and show fallback message |

## Topics

### Design

- [TUI Design System](./tui-design-skill.md) -- layout paradigms, interaction models, color systems, data visualization, design principles
- [App Patterns Gallery](./resources/app-patterns.md) -- real-world design analysis of lazygit, k9s, yazi, btop, zellij, and more
- [Visual Catalog](./resources/visual-catalog.md) -- box-drawing, block elements, braille, status indicators, tree characters

### Ratatui Core

- [Layout System](./layout-system.md) -- constraints, nested layouts, responsive design, ratatui-macros
- [Widgets](./widgets.md) -- Paragraph, List, Table, Block, Gauge, Tabs, Canvas, custom Widget trait
- [Styling](./styling.md) -- colors (named/indexed/RGB), modifiers, themes, Stylize trait
- [Backend System](./backend-system.md) -- Crossterm (default), Termion, Termwiz comparison and setup
- [Event Handling](./event-handling.md) -- keyboard, mouse, resize, mode-based routing, polling vs blocking
- [Macros](./macros.md) -- `ratatui-macros` DSL for layouts (`vertical!`, `horizontal!`) and styled text

### Ratatui Advanced

- [Async Integration](./async-integration.md) -- Tokio channels, streaming responses, loading states, cancellation
- [Scrolling](./scrolling.md) -- ListState, Paragraph scroll, scrollbars, page navigation, mouse scroll
- [Chat Applications](./chat-applications.md) -- bubbles, alignment, streaming, auto-scroll, multi-line input
- [Markdown Rendering](./markdown.md) -- tui-markdown, pulldown-cmark, syntax highlighting, caching
- [Code Editor](./code-editor.md) -- ratatui-code-editor, Tree-sitter highlighting, file I/O, search
- [Images](./images.md) -- ratatui-image, Kitty/Sixel/iTerm2/halfblock protocols, fallbacks
- [Prompts and Forms](./prompts.md) -- tui-prompts, multi-field forms, validation, autocomplete
- [Widget Collections](./widget-collections.md) -- tui-popup, tui-big-text, tui-scrollview, edtui, tui-logger
- [Web Deployment](./web-deployment.md) -- Ratzilla WASM backends (WebGL2, Canvas, DOM), Trunk, GitHub Pages

### Validation & Testing

- [Testing TUI Applications](./testing-a-tui.md) -- architectural separation, TestBackend, snapshot testing (insta), input simulation, CI/CD best practices

### Bubble Tea (Go)

- [Architecture](./bubble-tea-architecture.md) -- Elm pattern, messages/commands, state management, async, navigation
- [Components](./bubble-tea-components.md) -- Bubbles library (textinput, list, viewport, spinner) and Lip Gloss styling

### Cross-Framework

- [Best Practices](./tui-best-practices.md) -- keyboard conventions, layout design, performance, accessibility, error handling
- [Ratatui Architecture](./ratatui-architecture.md) -- minimal app structure and event loop pattern

## Resources

- [Ratatui Docs](https://docs.rs/ratatui) | [GitHub](https://github.com/ratatui-org/ratatui) | [Book](https://ratatui.rs)
- [Bubble Tea GitHub](https://github.com/charmbracelet/bubbletea) | [Charm.sh](https://charm.sh)
- [Awesome Ratatui](https://github.com/ratatui-org/awesome-ratatui)

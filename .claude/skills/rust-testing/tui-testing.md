# TUI Testing

For Ratatui and similar Rust TUIs, separate rendering tests from event/reducer tests.

## Rendering with `TestBackend`

Use `ratatui::backend::TestBackend` to render widgets or whole screens into a deterministic buffer.

```rust
use insta::assert_debug_snapshot;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn services_tab_renders() {
    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| render(frame, frame.area(), &app)).unwrap();

    let buffer = terminal.backend().buffer().clone();
    assert_debug_snapshot!(buffer);
}
```

Use at least:

- one normal-width render case
- one narrow-width render case that proves the code does not panic

## Event and Reducer Tests

Keep keyboard logic out of rendering assertions where possible. Test transition helpers and key handlers directly:

```rust
#[test]
fn escape_closes_modal_without_committing_changes() {
    handle_key(&mut app, key(KeyCode::Esc));
    assert!(app.modal.is_none());
    assert!(!app.dirty);
}
```

Prefer:

- pure reducers for state transformations
- thin modal handlers that delegate to reducers
- app-level tests for overview/detail transitions and modal stack behavior

## Buffer Assertions

For light-weight checks, inspect buffer content directly:

```rust
let content: String = terminal.backend().buffer().content
    .iter()
    .map(|cell| cell.symbol())
    .collect();

assert!(content.contains("Protect"));
```

Use snapshots when the layout matters, and direct assertions when only a few key strings or styles matter.

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

## Subprocess Hazards: Raw Mode Mutates the Shared `/dev/tty`

When integration tests spawn a TUI binary via `assert_cmd` (or any
`Command::spawn`), the subprocess inherits the test process's
controlling terminal. If the subprocess calls
`crossterm::enable_raw_mode()`, it opens `/dev/tty` and calls
`tcsetattr` on the **shared** controlling terminal — termios changes
are process-group-wide, not per-fd. Among other things, raw mode
disables `OPOST`, which is what translates `\n` to CRLF on output.

While the subprocess is alive, the parent test process's writes to
the same terminal lose their CR translation: every `\n` becomes a
bare LF (cursor down, no carriage return). Any redrawing UI in the
parent — nextest's progress bar, indicatif spinners — breaks because
its cursor-up + erase-line + re-emit sequence lands at the wrong
column on each redraw.

Two failure modes were observed in this monorepo:

| Symptom | Cause |
|---------|-------|
| "Running [00:00:NN]" lines staircase rightward across the screen | Subprocess wrote ANSI to `/dev/tty` (via a stdout redirect) AND held raw mode. Compound corruption. |
| Lines stack at column 0 but never overwrite | Subprocess held raw mode briefly; ANSI leak fixed but OPOST/CRLF still off during the subprocess's lifetime. |

### The "Reaches the Event Loop" Anti-Pattern

A tempting integration-test shape:

```rust
#[test]
fn flag_x_reaches_event_loop() {
    cargo_bin_cmd!("question")
        .args(["choose-one", "a", "b", "c", "--flag-x"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("question:"));
}
```

The expected mechanism is "`enable_raw_mode()` fails with ENXIO when
the test process has no controlling tty, exit 1, assertion passes."
This **only works in environments without a controlling tty** (CI,
detached runners). When a developer runs `just test` from an
interactive shell:

1. The test process has a controlling tty.
2. The subprocess inherits it.
3. `enable_raw_mode()` succeeds.
4. The prompt actually runs on the developer's terminal.
5. Either it hangs waiting on `event::read` (inline viewport — no
   terminal-driven rescue event), or it eventually exits but in the
   meantime corrupts the parent's redraw via the OPOST issue above.

Do not write tests like this. Coverage for CLI flag plumbing belongs in:

- `clap` derive-level unit tests (`Cli::try_parse_from(...)`)
- `--help` snapshot tests for flag visibility
- **Writer-seam unit tests** that inject a synthetic `run_prompt`
  closure (see the biscuit-tui skill for the canonical
  `run_with_writer` pattern).

### Required Guard for Interactive Binaries

Any binary that calls `enable_raw_mode()` should refuse to run when
it's clearly headless. The cleanest check is at the top of the
prompt-running function, before any `/dev/tty` operation:

```rust
use std::io::{self, IsTerminal};

if !io::stdout().is_terminal() && !io::stderr().is_terminal() {
    return Err(io::Error::new(
        io::ErrorKind::Other,
        "no interactive terminal available",
    ));
}
```

The two-stream check is deliberate: shell command substitution
(`FOO=$(question ...)`) pipes stdout but inherits stderr from the
terminal, so the redirect path still works. Only the test-harness
shape (both streams piped) is rejected. Without this guard, the
binary is a foot-gun for anyone running it in a pipeline or test.


# Spinner

A single-line activity spinner for long-running foreground work. It redraws one line in place about every 80 ms, accepts replacement text while running, and clears its line when finished.

`Spinner` is **not** a `TerminalRenderable` or render-tree node. It is a live, time-driven widget that owns a background thread and writes directly to its output, so it cannot be composed into other components or rendered to Markdown or the browser.

## Programmatic Use

```rust
use std::time::Duration;
use biscuit_terminal::prelude::*;

let spinner = Spinner::new("updating")
    .with_delay(Duration::from_millis(150))
    .start_on_stderr();

// ... long-running work ...
spinner.set_text("rate limited, using fallback method");

// Clears the spinner line; dropping the handle does the same.
spinner.finish();
```

### Key API

| Item | Description |
|------|-------------|
| `Spinner::new(text)` | Create a spinner showing `text` (a single line) |
| `.with_delay(Duration)` | Draw nothing until the delay has elapsed (default: none) |
| `.with_width(u32)` | Fix the line width instead of querying the terminal before each frame |
| `.start_on_stderr()` | Start on stderr; inert when stderr is not a terminal |
| `.start_on(writer, is_terminal)` | Start on any `Write + Send + 'static` writer |
| `SpinnerHandle::set_text(text)` | Replace the text shown by the next frame |
| `SpinnerHandle::finish()` | Stop the spinner and clear its line |
| `frame(index, text, width)` | The pure frame sequence for frame `index` |
| `CLEAR_LINE` | The clear sequence written when the spinner stops |

## Output Contract

- Output goes to stderr only, and only when stderr is a terminal. On a non-terminal, no thread is started and nothing is ever written.
- Nothing is drawn before the delay, so work that finishes quickly never flashes a spinner.
- Each frame is a carriage return, a glyph, the text, and an erase-to-end-of-line. Its visible width is at most one column less than the terminal width, so eager-wrapping consoles never move the cursor to a new line.
- Finishing or dropping the handle writes `CLEAR_LINE` exactly once, and only if at least one frame was drawn. A spinner stopped before its delay writes nothing.
- Write errors are ignored: the spinner never fails the work it accompanies.

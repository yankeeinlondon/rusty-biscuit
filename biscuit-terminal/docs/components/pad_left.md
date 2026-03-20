# PadLeft

Right-aligns content by adding spaces before it to guarantee a minimum width. If content is shorter than `min_width`, spaces are prepended. If content is longer, it passes through unchanged unless truncation is enabled, in which case characters are removed from the start (left side).

See also: [PadRight](pad_right.md) for left-aligned padding.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Pad to at least 10 characters (right-aligned)
let padded = PadLeft::new("hello", 10);
assert_eq!(padded.render_optimistic(Some(80)), "     hello");

// Content longer than min_width passes through unchanged
let padded = PadLeft::new("hello world", 5);
assert_eq!(padded.render_optimistic(Some(80)), "hello world");

// Truncate to exactly 10 characters (removes from the left/start)
let padded = PadLeft::new("hello world", 10).truncate();
assert_eq!(padded.render_optimistic(Some(80)), "ello world");

// Works with any RenderableContent
let padded = PadLeft::new(Prose::new("<bold>hi</bold>"), 20);
```

### Key API

| Method | Description |
|--------|-------------|
| `PadLeft::new(content, min_width)` | Create with content and minimum width |
| `.truncate()` | Enable truncation to exactly `min_width` characters |

## CLI

Exposed via `bt pad-left`:

```bash
bt pad-left "hello" --width 20
```

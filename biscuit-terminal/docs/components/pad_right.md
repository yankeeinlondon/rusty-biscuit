# PadRight

Left-aligns content by adding spaces after it to guarantee a minimum width. If content is shorter than `min_width`, spaces are appended. If content is longer, it passes through unchanged unless truncation is enabled, in which case characters are removed from the end (right side).

See also: [PadLeft](pad_left.md) for right-aligned padding.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Pad to at least 10 characters (left-aligned)
let padded = PadRight::new("hello", 10);
assert_eq!(padded.render_optimistic(Some(80)), "hello     ");

// Content longer than min_width passes through unchanged
let padded = PadRight::new("hello world", 5);
assert_eq!(padded.render_optimistic(Some(80)), "hello world");

// Truncate to exactly 10 characters (removes from the right/end)
let padded = PadRight::new("hello world", 10).truncate();
assert_eq!(padded.render_optimistic(Some(80)), "hello worl");

// Works with any RenderableContent
let padded = PadRight::new(Prose::new("<bold>hi</bold>"), 20);
```

### Key API

| Method | Description |
|--------|-------------|
| `PadRight::new(content, min_width)` | Create with content and minimum width |
| `.truncate()` | Enable truncation to exactly `min_width` characters |

## CLI

Exposed via `bt pad-right`:

```bash
bt pad-right "hello" --width 20
```

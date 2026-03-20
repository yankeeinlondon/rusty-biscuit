# TerminalImage

Renders images inline in the terminal using the Kitty graphics protocol with automatic fallback to iTerm2 inline images. Handles cursor positioning, scroll compensation, and terminal-specific quirks across Kitty, Ghostty, WezTerm, Warp, and iTerm2.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;

// Basic image display
let image = TerminalImage::new("photo.jpg").unwrap();
let term = Terminal::new();
println!("{}", image.display(&term));

// With width specification via path suffix
let image = TerminalImage::new("diagram.png|50%").unwrap();  // 50% of terminal
let image = TerminalImage::new("icon.png|25").unwrap();      // 25 characters wide
let image = TerminalImage::new("banner.png|fill").unwrap();  // Full width

// Parse width spec separately
let (path, width) = parse_filepath_and_width("photo.jpg|75%");
```

### Width Specifications

| Spec | Meaning |
|------|---------|
| `filename.jpg` | Default 50% width |
| `filename.jpg\|25` | Fixed 25 characters |
| `filename.jpg\|50%` | 50% of terminal width |
| `filename.jpg\|fill` | Fill available width |

### Key API

| Method/Function | Description |
|--------|-------------|
| `TerminalImage::new(path_with_width)` | Create from path (with optional `\|width` suffix) |
| `parse_filepath_and_width(str)` | Parse path and width spec separately |
| `parse_width_spec(str)` | Parse just the width portion |
| `calculate_display_dimensions(...)` | Calculate pixel dimensions for display |

### Image Options

Use `TerminalImageOptionsBuilder` for advanced configuration:

```rust
use biscuit_terminal::prelude::*;
use std::path::PathBuf;

let options = TerminalImageOptionsBuilder::default()
    .base_path(PathBuf::from("/safe/directory"))
    .max_file_size(5 * 1024 * 1024)  // 5MB limit
    .allow_remote(false)
    .build();
```

### Error Handling

Returns `TerminalImageError` with variants for file not found, invalid path, invalid width spec, path traversal blocked, file too large, remote URL blocked, image load errors, encoding errors, and unsupported terminal.

## CLI

Exposed via `bt image`:

```bash
bt image photo.jpg              # Default 50% width
bt image photo.jpg --width 75%  # 75% width
bt image banner.png --width fill
bt image icon.png --width 25    # 25 characters
```

Layout args are supported: `--margin-left`, `--margin-right`, `--alignment`, etc.

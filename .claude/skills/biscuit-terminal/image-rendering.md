# Image Rendering

`TerminalImage` renders inline images as escape-sequence strings (Kitty or iTerm2 protocol), with terminal-aware cursor management in `render_to_terminal()`.

## Basic Usage

```rust
use biscuit_terminal::components::terminal_image::TerminalImage;
use biscuit_terminal::terminal::Terminal;
use std::path::Path;

let term = Terminal::new();
let img = TerminalImage::new(Path::new("photo.jpg"))?;

let output = img.render_to_terminal(&term)?;
if output.is_empty() {
    // Unsupported terminals currently return an empty render string.
    println!("{}", img.generate_alt_text());
} else {
    print!("{}", output);
}
```

## Path and Width Validation

`TerminalImage::new()` validates that:
- The file exists
- The path can be canonicalized to a local absolute path

Width input is validated by `parse_width_spec()`:
- `50%` -> `ImageWidth::Percent(0.5)`
- `80` or `80ch` -> `ImageWidth::Characters(80)`
- `fill` -> `ImageWidth::Fill`

```rust
use biscuit_terminal::components::terminal_image::{parse_width_spec, TerminalImage};
use std::path::Path;

let width = parse_width_spec("75%")?;
let img = TerminalImage::new(Path::new("photo.jpg"))?.with_width(width);
```

## Policy Controls with TerminalImageOptions

`TerminalImageOptions` is a policy helper (`base_path`, `max_file_size`, `allow_remote`, `width`) that your application can enforce before rendering.

Current runtime flow (`TerminalImage::new()`, `from_spec()`, `render_to_terminal()`) does **not** automatically enforce remote/path/size policy checks.

### TerminalImageOptions Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `base_path` | `Option<PathBuf>` | `None` | Optional directory boundary for allowed files |
| `max_file_size` | `u64` | `10MB` | Maximum allowed file size |
| `allow_remote` | `bool` | `false` | Whether URL-like inputs are allowed |
| `width` | `ImageWidth` | `Percent(0.5)` | Default width policy |

### App-Enforced Policy Example

```rust
use biscuit_terminal::components::image_options::{ImageWidth, TerminalImageOptions};
use biscuit_terminal::components::terminal_image::TerminalImage;
use std::path::{Path, PathBuf};

let options = TerminalImageOptions::builder()
    .base_path(PathBuf::from("/safe/images"))
    .max_file_size(5 * 1024 * 1024)
    .allow_remote(false)
    .width(ImageWidth::Percent(0.6))
    .build();

let path = Path::new("photo.jpg");
let file_size = std::fs::metadata(path)?.len();

if !options.is_size_allowed(file_size) || !options.is_path_allowed(path) {
    panic!("Image blocked by policy");
}

let img = TerminalImage::new(path)?.with_width(options.width.clone());
```

## Width Syntax

```
image.png           -> Default 50% width
image.png|25%       -> 25% of terminal width
image.png|80        -> Fixed 80 columns
image.png|80ch      -> Fixed 80 columns
image.png|fill      -> Fill available width
```

## Protocol Selection and Cursor Behavior

`render_to_terminal()` selects protocol and terminal-specific cursor handling:

| Terminal | Protocol | Sizing Strategy | Cursor Behavior |
|----------|----------|-----------------|-----------------|
| Kitty/Ghostty/Konsole | Kitty (`c=` only) | Terminal derives rows | Auto-advance with overshoot correction |
| WezTerm | Kitty (`c=` + `r=`) | Explicit rows required | Explicit row advance + carriage return |
| iTerm2 | iTerm2 OSC 1337 | Native scaling (`preserveAspectRatio=1`) | Overshoot correction + row advance |
| Warp | Kitty (`c=` only) | Terminal derives rows | Uses floor-based row count to avoid blank-line overshoot |
| Others | None | N/A | Returns empty output |

`render_to_terminal()` normalizes back to column 0 with `\r`. It usually avoids trailing newlines, but may append one `\n` when bottom-of-screen scroll compensation is needed.

## Direct Protocol Methods

For lower-level composition paths:

```rust
// Includes its own cursor advancement
let kitty = img.render_as_kitty(80)?;
let iterm = img.render_as_iterm2(80)?;

// Raw protocol payload constructors
let png_data = img.encode_as_png(&loaded_image)?;
let kitty_escape = img.render_kitty_cells(&png_data, width_cells, height_cells);
let iterm_escape = img.render_iterm2(&png_data, "40", "filename.png");
```

## Cell Size and Aspect Ratio

`TerminalImage` uses measured cell size when available (`discovery::fonts::cell_size()`), with an `8x16` fallback. This avoids visibly distorted images on terminals with non-2:1 cell geometry.

## Error Handling Notes

`TerminalImageError` includes:
- File/path/encoding/runtime errors (`FileNotFound`, `InvalidPath`, `ImageLoadError`, etc.)
- Policy-style variants (`PathTraversalBlocked`, `FileTooLarge`, `RemoteUrlBlocked`)

The policy-style variants are primarily tied to helper checks; they are not all emitted by the default `new()/render_to_terminal()` path.

## Fallback Behavior Clarification

- `render_to_terminal()` returns `Ok(String::new())` when `ImageSupport::None`
- `fallback_render()` currently delegates to `render_to_terminal()` and therefore also yields empty output in that case
- If you want textual fallback, print `generate_alt_text()` from your application

## Related

- [Terminal Struct](./terminal-struct.md) - `ImageSupport` detection
- [Discovery Functions](./discovery.md) - `image_support()` and terminal detection

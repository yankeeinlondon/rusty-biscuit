# TerminalImage

Renders images inline in the terminal in the format chosen by the terminal's [graphics route](../discovery/graphics-routes.md) (Kitty or iTerm2 today), and as alt text when the route has no usable format. Handles cursor positioning, scroll compensation, and terminal-specific quirks across Kitty, Ghostty, WezTerm, Warp, and iTerm2.

## Programmatic Use

```rust
use biscuit_terminal::prelude::*;
use std::path::Path;

// Basic image display
let image = TerminalImage::new(Path::new("photo.jpg")).unwrap();
let term = Terminal::new();
println!("{}", image.display(&term));

// With width specification via path suffix (`from_spec` parses it; `new` does not)
let image = TerminalImage::from_spec("diagram.png|50%").unwrap();  // 50% of terminal
let image = TerminalImage::from_spec("icon.png|25").unwrap();      // 25 characters wide
let image = TerminalImage::from_spec("banner.png|fill").unwrap();  // Full width

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

`ImageWidth::Scale(f32)` has no string form. It sizes an image from its natural width relative to terminal text: pixels per unit = scale × cell height ÷ 16, and columns = natural width × pixels per unit ÷ cell width, rounded up and clamped to the available columns. At `1.0`, 16 units (Mermaid's body text) are one terminal line tall. A raster image's natural width is its pixel width; `biscuit-visualized`'s `MermaidDiagram` passes the SVG's measured width. An unknown cell size (native Windows, CI, every stream redirected) uses `CellSize::FALLBACK` (8×16).

### Key API

| Method/Function | Description |
|--------|-------------|
| `TerminalImage::new(&Path)` | Create from a file path; the file must exist |
| `TerminalImage::from_spec(str)` | Create from a path with an optional `\|width` suffix |
| `TerminalImage::from_encoded_png(EncodedPng)` | Create from PNG bytes the program produced itself; see [Already-encoded PNG input](#already-encoded-png-input) |
| `TerminalImage::render_with_route(&Terminal)` | Render for the terminal's graphics route, as pixels or fallback text; see [Graphics routes](#graphics-routes) |
| `parse_filepath_and_width(str)` | Parse path and width spec separately |
| `parse_width_spec(str)` | Parse just the width portion |
| `calculate_display_dimensions(...)` | Calculate pixel dimensions for display |
| `ImageWidth::scaled_columns(scale, natural_width, cell)` | Columns for `ImageWidth::Scale`, before the clamp |
| `TerminalImage::resolve_scaled_dimensions_for(...)` | Columns, margins, and offset for content with a known natural width |

### Graphics routes

`TerminalImage` makes no format decision of its own. It follows `Terminal::graphics_route`, which `Terminal::new()` resolves once from the environment (and which a caller can supply through `Terminal::builder()`); [Graphics Routes](../discovery/graphics-routes.md) explains how the route is chosen and overridden with `BISCUIT_GRAPHICS`.

| Route format | What is sent |
|--------------|--------------|
| Kitty | Kitty graphics, except to iTerm2, which gets its own format unless `BISCUIT_GRAPHICS=kitty` forced Kitty |
| iTerm2 | iTerm2's `OSC 1337;File=` format |
| Sixel | The text fallback; the Sixel encoder is **planned** |
| none | The text fallback |

`render_with_route` returns an `ImageOutput` so a caller can tell the two outcomes apart:

```rust
use biscuit_terminal::components::terminal_image::ImageOutput;
use biscuit_terminal::prelude::*;
use std::path::Path;

let term = Terminal::new();
let image = TerminalImage::new(Path::new("photo.jpg"))?.with_alt_text("A photo");
match image.render_with_route(&term)? {
    ImageOutput::Pixels(sequence) => print!("{sequence}"),
    ImageOutput::Fallback { text, reason } => println!("{text}  ({reason})"),
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

- **Fallback text** is the alt text, or `[Image: <file name>]` when none was set, with control characters removed. A route without a usable format used to render an empty string; it now renders this text. A half-block rendering is **planned** as the first fallback.
- **Errors come first.** The image is loaded and decoded before anything is produced, so a missing, unreadable, or corrupt image is an error even when the route has no pixels, rather than becoming alt text.
- `render()` and `display()` call `render_with_route` and print either outcome; they turn an error into an empty string (logged as a warning).

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

### Already-encoded PNG input

When your program has already produced a PNG in memory, such as a diagram rasterized at its display size, wrap the bytes in an `EncodedPng` and build the image with `TerminalImage::from_encoded_png`. Kitty and iTerm2 then receive those exact bytes; nothing is decoded and encoded again, and the image is not written to a file first.

```rust
use biscuit_terminal::components::terminal_image::EncodedPng;
use biscuit_terminal::prelude::*;

let png = EncodedPng::new(std::fs::read("diagram.png")?)?;
let image = TerminalImage::from_encoded_png(png)
    .with_width(ImageWidth::Characters(60))
    .with_alt_text("Request flow");
println!("{}", image.display(&Terminal::new()));
# Ok::<(), Box<dyn std::error::Error>>(())
```

`EncodedPng::new` validates the bytes before anything is decoded or any terminal rows are reserved, and fails with:

| Check | Error |
|-------|-------|
| More than 10 MiB (the default `TerminalImageOptions::max_file_size`) | `TerminalImageError::FileTooLarge` |
| Not a readable PNG header | `TerminalImageError::ImageLoadError` |
| Pixel dimensions beyond the `image` crate's default 512 MiB decode allocation limit | `TerminalImageError::ImageLoadError` (a limits error) |

`bytes()`, `width()`, and `height()` return the bytes and the pixel size read from the header.

The bytes are sent unchanged by the standalone Kitty and iTerm2 paths, the inline (`TwoColumn`) path, and `render_as_kitty`. They are decoded only by `load_image()`, or by an actual resize: `render_as_iterm2` resizes when the target pixel width (columns × cell width) differs from the PNG's width. To skip that resize, produce the PNG at exactly the width it will be shown at, as `biscuit-visualized`'s diagram components do.

`EncodedPng` is meant for images the program produced itself. Caller-supplied files should still go through the path constructors (`new`, `from_spec`), which require the file to exist and resolve to a local path, together with whatever `TerminalImageOptions` policy your application enforces. An `EncodedPng` image has no file name, so iTerm2 receives `name=image.png` and the alt text is `[Image: image]` unless you set `with_alt_text`.

### PNG Encoding

Whenever `TerminalImage` has to encode pixels itself (a file decoded from disk, or a resize), `encode_as_png` uses zlib's default compression level with adaptive filtering (`PngEncoder::new_with_quality(CompressionType::Default, FilterType::Adaptive)`). The `image` crate's plain `write_to` favors speed and can produce a PNG larger than the raw pixels; with these settings the 400×400 iTerm2 app icon, for example, encodes to about 43 KB instead of 53 KB. Smaller payloads mean less data written to the terminal for every image.

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

When the terminal has no usable graphics route, `bt image` prints the fallback text, and `bt` explains why on stderr when the reason has a known fix; see [Hints](../discovery/graphics-routes.md#hints).

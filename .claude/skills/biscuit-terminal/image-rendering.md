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

`ImageWidth::Scale(f32)` has no string form. It sizes content from its natural width relative to terminal text (pixels per unit = scale × cell height ÷ 16, columns rounded up, clamped to the available columns). Resolve it with `TerminalImage::resolve_scaled_dimensions_for(width, layout, term_width, natural_width, cell)`. The plain `resolve_dimensions_for` has no natural width, so there it behaves like `Fill`. Use `CellSize::FALLBACK` (8×16) for an unknown cell size rather than a new literal.

```rust
use biscuit_terminal::components::terminal_image::{parse_width_spec, TerminalImage};
use std::path::Path;

let width = parse_width_spec("75%")?;
let img = TerminalImage::new(Path::new("photo.jpg"))?.with_width(width);
```

## Already-encoded PNG (`EncodedPng`)

For PNG bytes the program produced itself (e.g. `biscuit-visualized`'s diagram
components, which rasterize at the display size), skip the file round-trip:

```rust
use biscuit_terminal::components::terminal_image::{EncodedPng, ImageWidth, TerminalImage};

let png = EncodedPng::new(png_bytes)?; // validates; no decode
let img = TerminalImage::from_encoded_png(png).with_width(ImageWidth::Characters(60));
```

- `EncodedPng::new(Vec<u8>)` checks, before any decode or row reservation:
  ≤ 10 MiB (`DEFAULT_MAX_FILE_SIZE`, else `FileTooLarge`), a readable PNG
  header (else `ImageLoadError`), and dimensions within the `image` crate's
  default 512 MiB decode allocation limit (else `ImageLoadError` with a limits
  error). Accessors: `bytes()`, `width()`, `height()` (from the header).
- Kitty and iTerm2 standalone paths, the inline (`TwoColumn`) path, and
  `render_as_kitty` send the bytes **unchanged**. They are decoded only by
  `load_image()` or an actual resize (`render_as_iterm2` when the target pixel
  width, columns × cell width, differs from the PNG width). Internally the
  protocol paths go through `ImagePixels` (`Encoded` borrows the bytes,
  `Decoded` encodes a loaded file).
- No file: iTerm2 `name=` is `image.png`, alt text `[Image: image]`, unless
  `with_alt_text` is set.
- Not for caller-supplied files; those keep using `new` / `from_spec`.

## PNG encoding

`encode_as_png` (and every internal encode: decoded files, resizes) uses
`PngEncoder::new_with_quality(CompressionType::Default, FilterType::Adaptive)`
(`encoded_png::encode_png`), not `write_to`, whose fast settings can exceed
the raw pixel size. Measured once on macOS: the 400x400 iTerm2 app icon went
from 53,114 to 43,044 bytes.

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

The policy-style variants are primarily tied to helper checks; they are not all emitted by the default `new()`/`render_with_route()` path.

## Graphics Route and Fallback

- The encoder follows `Terminal::graphics_route` (a `GraphicsRoute`: format,
  transport, placement, confidence, structured `RouteReason`,
  `needs_measured_cells`), resolved once
  per `Terminal`; never re-resolve per image. Developer guide:
  `biscuit-terminal/docs/discovery/graphics-routes.md`.
- `render_with_route(&term)` returns `ImageOutput::Pixels(seq)` or
  `ImageOutput::Fallback { text, reason }` (alt text with controls stripped).
  Load/decode errors stay `Err` either way; `render()` maps errors to `""`.
- A Kitty route to `TerminalApp::ITerm2` uses the iTerm2 format unless the
  route is `Forced` (iTerm2 ignores `q=2` and places nothing for width-only `c=`).
- A Sixel route falls back (encoder not built yet); a route with no pixels
  falls back with the route's reason.
- Transport `Tmux`: `protocol::for_transport` wraps every APC/OSC/DCS in its
  own envelope (`terminal_image/tmux.rs`, `wrap_graphics_for_tmux`; CSI such
  as `ESC[s`/`ESC[u`/mode 2026 stays unwrapped, an existing envelope is never
  re-wrapped). Kitty uploads through tmux use `render_kitty_with_id`
  (`i=` from `probe::next_kitty_id`, `q=2`, `c=`+`r=`). Any wrapped message
  over `TMUX_MESSAGE_BUDGET` (1 MiB) → `Fallback` (`BlockedTransport`, size in
  the message) before rows are reserved; `render_inline` returns
  `UnsupportedTerminal` instead.
- A `needs_measured_cells` route (Zellij) falls back when `term.cell_size()`
  is `None` (`unmeasured_cells_reason`); never size such a route from the
  8×16 estimate. The library never prints hints; `bt`
  does (stderr, terminal only, once per reason, only with a `fix`).
- Tests: assert pixel dispatch through `protocol::encoder_for` (rendering
  pixels queries the live cursor), and build routes with
  `resolve_graphics_route(&RouteInputs::new(true).with_var(..))`, never by
  mutating the process env.

## Related

- [Terminal Struct](./terminal-struct.md) - `graphics_route` and its `ImageSupport` projection
- [Discovery Functions](./discovery.md) - `graphics_route()` / `resolve_graphics_route()`

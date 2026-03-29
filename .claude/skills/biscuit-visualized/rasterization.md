# Rasterization

SVG-to-PNG conversion using resvg.

## API

### Rasterize from bytes

```rust
use biscuit_visualized::raster::rasterize_svg_to_png_bytes;

let svg_data = std::fs::read("diagram.svg")?;
let png_bytes = rasterize_svg_to_png_bytes(&svg_data, 2.0)?;
std::fs::write("diagram.png", &png_bytes)?;
```

### Rasterize file-to-file

```rust
use biscuit_visualized::raster::rasterize_svg;

rasterize_svg(
    Path::new("input.svg"),
    Path::new("output.png"),
    2.0,  // scale factor
)?;
```

### Query available fonts

```rust
use biscuit_visualized::raster::available_font_families;

let fonts = available_font_families();
// Returns Vec<String> of system font family names
// e.g., ["Arial", "Helvetica", "Courier New", ...]
```

## Scale Factor

The `scale` parameter controls the DPI multiplier for PNG output:

| Scale | Effective DPI | Use Case |
|-------|---------------|----------|
| 1.0 | 96 DPI | Standard displays |
| 2.0 | 192 DPI | Retina/HiDPI displays |
| 3.0 | 288 DPI | High-quality print |

Higher scales produce larger PNG files with more detail.

## Font Database

The system font database is loaded lazily on first use and cached via `OnceLock` for thread safety. This means:

- First rasterization call may be slightly slower (font enumeration)
- Subsequent calls reuse the cached font database
- No need to manually initialize or manage fonts

resvg uses the system font database to render text in SVGs. If a font referenced in the SVG is not installed, resvg falls back to a default font.

## RasterError

```rust
pub enum RasterError {
    /// SVG data could not be parsed
    SvgParseFailed(String),
    /// Rendering to pixmap failed
    RenderFailed(String),
    /// File I/O error
    IoError(#[from] std::io::Error),
}
```

Common causes:
- `SvgParseFailed`: Malformed SVG, unsupported SVG features
- `RenderFailed`: Zero-size viewport, extremely large dimensions

## Source Files

| File | Contents |
|------|----------|
| `biscuit-visualized/src/raster/mod.rs` | Font database initialization, `available_font_families()` |
| `biscuit-visualized/src/raster/png.rs` | `rasterize_svg_to_png_bytes()`, `rasterize_svg()`, `RasterError` |
